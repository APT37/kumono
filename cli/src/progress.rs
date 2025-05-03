use anyhow::Result;
use indicatif::{ ProgressBar, ProgressStyle };
use num_format::{ Locale, ToFormattedString };
use size::Size;
use std::{ fmt, process, time::Duration };
use tokio::sync::mpsc::Receiver;

use crate::cli::ARGS;

pub fn n_fmt(n: usize) -> String {
    n.to_formatted_string(&Locale::en)
}

#[derive(Clone)]
pub enum DownloadState {
    Failure(Size, String),
    Skip,
    Success(Size),
}

#[derive(Default, Clone)]
struct Stats {
    success: usize,
    skipped: usize,
    failure: usize,
    dl_size: i64,
    errors: Vec<String>,
}

impl Stats {
    #[allow(clippy::needless_pass_by_value)]
    fn update(&mut self, download_state: DownloadState) {
        match download_state {
            DownloadState::Failure(size, err) => {
                self.dl_size += size.bytes();
                self.failure += 1;

                self.errors.dedup();
                if self.errors.len() == (ARGS.max_errors.clamp(1, 10) as usize) {
                    self.errors.remove(0);
                }
                self.errors.push(err);
                self.errors.dedup();
            }
            DownloadState::Skip => {
                self.skipped += 1;
            }
            DownloadState::Success(size) => {
                self.dl_size += size.bytes();
                self.success += 1;
            }
        }
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "downloaded approx. {} / success: {} / skipped: {} / failure: {}",
            Size::from_bytes(self.dl_size),
            n_fmt(self.success),
            n_fmt(self.skipped),
            n_fmt(self.failure)
        )
    }
}

pub fn bar(mut rx: Receiver<DownloadState>, length: u64) -> Result<()> {
    let bar = ProgressBar::new(length);

    bar.enable_steady_tick(Duration::from_millis(200));

    bar.set_style(
        ProgressStyle::with_template(
            "{prefix}[{elapsed_precise}] {bar:40.cyan/blue} {human_pos:>7}/{human_len:7} ({percent}%) {msg}"
        )?.progress_chars("##-")
    );

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
