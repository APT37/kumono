use crate::cli::ARGS;
use anyhow::Result;
use indicatif::{ HumanBytes, ProgressBar, ProgressStyle };
use num_format::{ Locale, ToFormattedString };
use std::{ fmt, fs::File, io::Write, path::PathBuf, process::exit, time::Duration };
use tokio::sync::mpsc::Receiver;

pub fn n_fmt(n: u64) -> String {
    n.to_formatted_string(&Locale::en)
}

#[derive(Clone)]
pub enum DownloadState {
    Success(u64, String),
    Skip(String),
    Failure(u64, String),
}

struct Stats {
    success: u64,
    skipped: u64,
    failure: u64,
    dl_size: u64,
    errors: Vec<String>,
    archive: Option<File>,
}

impl Stats {
    pub fn new(archive_path: &PathBuf) -> Self {
        Self {
            success: 0,
            skipped: 0,
            failure: 0,
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

    fn write_to_archive(&mut self, hash: String) {
        if let Some(ref mut archive) = self.archive {
            if let Err(err) = archive.write_all((hash + "\n").as_bytes()) {
                self.errors.push(err.to_string());
                exit(1);
            }
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn update(&mut self, download_state: DownloadState) {
        match download_state {
            DownloadState::Failure(size, err) => {
                self.dl_size += size;
                self.failure += 1;

                if self.errors.len() == 3 {
                    self.errors.remove(0);
                }
                self.errors.push(err);
            }
            DownloadState::Skip(hash) => {
                self.skipped += 1;

                if ARGS.download_archive {
                    self.write_to_archive(hash);
                }
            }
            DownloadState::Success(size, hash) => {
                self.dl_size += size;
                self.success += 1;

                if ARGS.download_archive {
                    self.write_to_archive(hash);
                }
            }
        }
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "downloaded approx. {} / finished: {} / skipped: {} / failed: {}",
            HumanBytes(self.dl_size),
            n_fmt(self.success),
            n_fmt(self.skipped),
            n_fmt(self.failure)
        )
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn bar(mut rx: Receiver<DownloadState>, archive: PathBuf, length: u64) -> Result<()> {
    let bar = ProgressBar::new(length);

    bar.set_style(
        ProgressStyle::with_template(
            "{prefix}[{elapsed_precise}] {bar:40.cyan/blue} {human_pos:>7}/{human_len:7} ({percent}%) {msg}"
        )?.progress_chars("##-")
    );

    bar.enable_steady_tick(Duration::from_millis(200));

    let mut stats = Stats::new(&archive);

    while let Some(dl_state) = rx.blocking_recv() {
        stats.update(dl_state);

        if !stats.errors.is_empty() {
            bar.set_message(format!("\n{}", stats.errors.join("\n")));
        }

        bar.inc(1);

        bar.set_prefix(stats.to_string());
    }

    bar.finish();

    if stats.failure > 0 {
        exit(1);
    }

    Ok(())
}
