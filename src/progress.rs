use crate::{ cli::ARGUMENTS, pretty::{ n_fmt, with_noun } };
use indicatif::{ HumanBytes, ProgressBar, ProgressStyle };
use itertools::Itertools;
use std::{
    collections::HashMap,
    fmt::{ Display, Formatter, Result },
    fs::File,
    io::Write,
    path::PathBuf,
    process::exit,
    sync::atomic::{ AtomicBool, Ordering::Relaxed },
    time::Duration,
};
use tokio::sync::mpsc::Receiver;

#[derive(Clone)]
pub enum DownloadAction {
    Start,
    Wait,
    Continue,
    ReportSize(u64),
    ReportLegacyHashSkip(String),
    Skip(Option<String>, Option<String>),
    Fail(String, Option<String>),
    Complete(Option<String>, Option<String>),
    Panic(String),
}

struct Stats {
    queued: u64,
    waiting: u64,
    active: u64,
    complete: u64,
    skipped: u64,
    failed: u64,
    dl_bytes: u64,
    errors: Vec<String>,
    panic: bool,
    archive_file: Option<File>,
    files_by_type: HashMap<String, usize>,
}

impl Stats {
    pub fn new(files: u64, archive_path: &PathBuf, files_by_type: HashMap<String, usize>) -> Self {
        Self {
            queued: files,
            waiting: 0,
            active: 0,
            complete: 0,
            skipped: 0,
            failed: 0,

            dl_bytes: 0,

            errors: Vec::with_capacity(3),

            panic: false,

            archive_file: if ARGUMENTS.download_archive {
                Some(Self::open_archive(archive_path))
            } else {
                None
            },

            files_by_type,
        }
    }

    fn open_archive(path: &PathBuf) -> File {
        File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(path)
            .unwrap_or_else(|err| {
                eprintln!("failed to open archive file {path}: {err}", path = path.display());
                exit(5);
            })
    }

    fn write_to_archive(&mut self, hash: Option<String>) {
        if
            ARGUMENTS.download_archive &&
            let Some(hash) = hash &&
            let Some(ref mut archive) = self.archive_file &&
            let Err(err) = archive.write_all((hash + "\n").as_bytes())
        {
            eprintln!("{err}");
            exit(6);
        }
    }

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
                self.dl_bytes += size;
                false
            }
            DownloadAction::ReportLegacyHashSkip(file_name) => {
                if self.errors.len() == 3 {
                    self.errors.remove(0);
                }
                self.errors.push(format!("skipped hash verification for legacy file: {file_name}"));
                false
            }
            DownloadAction::Skip(hash, extension) => {
                self.active -= 1;
                self.skipped += 1;
                self.detract_one_from_file_counter(extension);
                self.write_to_archive(hash);
                true
            }
            DownloadAction::Fail(error, extension) => {
                self.active -= 1;
                self.failed += 1;
                self.detract_one_from_file_counter(extension);
                if self.errors.len() == 3 {
                    self.errors.remove(0);
                }
                self.errors.push(error);
                true
            }
            DownloadAction::Complete(hash, extension) => {
                self.active -= 1;
                self.complete += 1;
                self.detract_one_from_file_counter(extension);
                self.write_to_archive(hash);
                true
            }
            DownloadAction::Panic(error) => {
                if self.errors.len() == 3 {
                    self.errors.remove(0);
                }
                self.errors.push(error);
                self.panic = true;
                false
            }
        }
    }

    fn detract_one_from_file_counter(&mut self, extension: Option<String>) {
        *self.files_by_type
            .entry(extension.unwrap_or_else(|| "unknown".to_string()))
            .or_default() -= 1;
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(
            f,
            "downloaded {} / {} queued / {} waiting / {} active / {} complete / {} skipped / {} failed{}",
            HumanBytes(self.dl_bytes),
            n_fmt(self.queued),
            n_fmt(self.waiting),
            n_fmt(self.active),
            n_fmt(self.complete),
            n_fmt(self.skipped),
            n_fmt(self.failed),
            if self.files_by_type.is_empty() {
                let mut buffer = Vec::with_capacity(self.files_by_type.len());

                buffer.push(
                    format!(
                        "\n{} left",
                        with_noun(self.queued + self.waiting + self.active, "file")
                    )
                );

                for (key, value) in self.files_by_type
                    .iter()
                    .filter(|(_, v)| **v != 0)
                    .sorted() {
                    buffer.push(format!("{key}: {value}", value = n_fmt(*value as u64)));
                }

                buffer.join(" / ")
            } else {
                String::new()
            }
        )
    }
}

pub static DOWNLOADS_FAILED: AtomicBool = AtomicBool::new(false);

#[allow(clippy::needless_pass_by_value)]
pub fn progress_bar(
    files: u64,
    archive: PathBuf,
    mut msg_rx: Receiver<DownloadAction>,
    last_target: bool,
    files_by_type: HashMap<String, usize>
) {
    let bar = ProgressBar::new(files);

    bar.set_style(
        ProgressStyle::with_template(
            "{prefix}[{elapsed_precise}] {bar:40.cyan/blue} {human_pos:>7}/{human_len:7} ({percent}%) {msg}"
        )
            .unwrap()
            .progress_chars("##-")
    );

    bar.enable_steady_tick(Duration::from_millis(200));

    let mut stats = Stats::new(files, &archive, files_by_type);

    while let Some(state) = msg_rx.blocking_recv() {
        if stats.update(state) {
            bar.inc(1);
        }

        if !stats.errors.is_empty() {
            bar.set_message(format!("\n{}", stats.errors.join("\n")));
        }

        bar.set_prefix(stats.to_string());

        if stats.panic {
            bar.finish();
            exit(7);
        }
    }

    bar.finish();

    if !last_target {
        eprintln!("\n");
    }

    if stats.failed != 0 {
        DOWNLOADS_FAILED.store(true, Relaxed);
    }
}
