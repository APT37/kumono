use crate::{ cli::ARGS, pretty::n_fmt };
use anyhow::Result;
use indicatif::{ HumanBytes, ProgressBar, ProgressStyle };
use std::{ fmt, fs::File, io::Write, path::PathBuf, process::exit, time::Duration };
use tokio::sync::mpsc::Receiver;

#[derive(Clone)]
pub enum DownloadAction {
    Start,
    Wait,
    Continue,
    ReportSize(u64),
    LegacyHashSkip(String),
    Skip(Option<String>),
    Fail(String),
    Complete(Option<String>),
}

struct Stats {
    queued: u64,
    waiting: u64,
    active: u64,
    complete: u64,
    skipped: u64,
    failed: u64,
    dl_size: u64,
    errors: Vec<String>,
    archive: Option<File>,
}

impl Stats {
    pub fn new(files: u64, archive_path: &PathBuf) -> Self {
        Self {
            queued: files,
            waiting: 0,
            active: 0,
            complete: 0,
            skipped: 0,
            failed: 0,

            dl_size: 0,

            errors: Vec::new(),

            archive: if ARGS.download_archive {
                Some(Self::open_archive(archive_path))
            } else {
                None
            },
        }
    }

    fn open_archive(path: &PathBuf) -> File {
        File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(path)
            .unwrap_or_else(|err| {
                eprintln!("failed to open archive file {}: {err}", path.display());
                exit(1);
            })
    }

    fn write_to_archive(&mut self, hash: Option<String>) {
        if ARGS.download_archive {
            if let Some(hash) = hash {
                if let Some(ref mut archive) = self.archive {
                    if let Err(err) = archive.write_all((hash + "\n").as_bytes()) {
                        self.errors.push(err.to_string());
                        exit(1);
                    }
                }
            }
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn update(&mut self, download_state: DownloadAction) -> bool {
        match download_state {
            DownloadAction::Start => {
                self.queued -= 1;
                self.active += 1;
                false
            }
            DownloadAction::Wait => {
                self.active -= 1;
                self.waiting += 1;
                false
            }
            DownloadAction::Continue => {
                self.waiting -= 1;
                self.active += 1;
                false
            }
            DownloadAction::ReportSize(size) => {
                self.dl_size += size;
                false
            }
            DownloadAction::LegacyHashSkip(name) => {
                if self.errors.len() == 3 {
                    self.errors.remove(0);
                }
                self.errors.push(format!("skipped hash verification for legacy file: {name}"));
                false
            }
            DownloadAction::Skip(hash) => {
                self.active -= 1;
                self.skipped += 1;
                self.write_to_archive(hash);
                true
            }
            DownloadAction::Fail(err) => {
                self.active -= 1;
                self.failed += 1;
                if self.errors.len() == 3 {
                    self.errors.remove(0);
                }
                self.errors.push(err);
                true
            }
            DownloadAction::Complete(hash) => {
                self.active -= 1;
                self.complete += 1;
                self.write_to_archive(hash);
                true
            }
        }
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "downloaded {} / {} queued / {} waiting / {} active / {} complete / {} skipped / {} failed",
            HumanBytes(self.dl_size),
            n_fmt(self.queued),
            n_fmt(self.waiting),
            n_fmt(self.active),
            n_fmt(self.complete),
            n_fmt(self.skipped),
            n_fmt(self.failed)
        )
    }
}

static mut DOWNLOADS_FAILED: bool = false;

pub fn downloads_failed() -> bool {
    unsafe { DOWNLOADS_FAILED }
}

#[allow(clippy::needless_pass_by_value)]
pub fn bar(
    files: u64,
    archive: PathBuf,
    mut msg_rx: Receiver<DownloadAction>,
    last_target: bool
) -> Result<()> {
    let bar = ProgressBar::new(files);

    bar.set_style(
        ProgressStyle::with_template(
            "{prefix}[{elapsed_precise}] {bar:40.cyan/blue} {human_pos:>7}/{human_len:7} ({percent}%) {msg}"
        )?.progress_chars("##-")
    );

    bar.enable_steady_tick(Duration::from_millis(200));

    let mut stats = Stats::new(files, &archive);

    while let Some(state) = msg_rx.blocking_recv() {
        if stats.update(state) {
            bar.inc(1);
        }

        if !stats.errors.is_empty() {
            bar.set_message(format!("\n{}", stats.errors.join("\n")));
        }

        bar.set_prefix(stats.to_string());
    }

    bar.finish();

    if !last_target {
        eprintln!("\n");
    }

    if stats.failed > 0 {
        unsafe {
            DOWNLOADS_FAILED = true;
        }
    }

    Ok(())
}
