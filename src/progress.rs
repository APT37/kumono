// use crate::cli::ARGS;
use anyhow::Result;
use indicatif::{ HumanBytes, ProgressBar, ProgressStyle };
use num_format::{ Locale, ToFormattedString };
use std::{ fmt, process, time::Duration };
use tokio::sync::mpsc::Receiver;

pub fn n_fmt(n: u64) -> String {
    n.to_formatted_string(&Locale::en)
}

#[derive(Clone)]
pub enum DownloadState {
    Failure(u64, String),
    Skip,
    Success(u64),
}

#[derive(Default, Clone)]
struct Stats {
    success: u64,
    skipped: u64,
    failure: u64,
    dl_size: u64,
    errors: Vec<String>,
}

impl Stats {
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
            DownloadState::Skip => {
                self.skipped += 1;
            }
            DownloadState::Success(size) => {
                self.dl_size += size;
                self.success += 1;
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

pub fn bar(mut rx: Receiver<DownloadState>, length: u64) -> Result<()> {
    let bar = ProgressBar::new(length);

    bar.set_style(
        ProgressStyle::with_template(
            "{prefix}[{elapsed_precise}] {bar:40.cyan/blue} {human_pos:>7}/{human_len:7} ({percent}%) {msg}"
        )?.progress_chars("##-")
    );

    bar.enable_steady_tick(Duration::from_millis(200));

    let mut stats = Stats::default();

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
        process::exit(1);
    }

    Ok(())
}
