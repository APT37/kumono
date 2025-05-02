use crate::n_fmt;
use size::Size;
use std::fmt;

pub enum DownloadState {
    Failure(Size, Option<String>),
    Skip,
    Success(Size),
}

#[derive(Default, Clone, Copy)]
pub struct Stats {
    pub success: usize,
    pub skipped: usize,
    pub failure: usize,
    pub dl_size: i64,
}

impl Stats {
    #[allow(clippy::needless_pass_by_value)]
    pub fn update(&mut self, state: DownloadState) {
        match state {
            DownloadState::Failure(size, _) => self.add_failure(size),
            DownloadState::Skip => self.add_skipped(),
            DownloadState::Success(size) => self.add_success(size),
        }
    }

    fn add_failure(&mut self, size: Size) {
        self.failure += 1;
        self.dl_size += size.bytes();
    }

    fn add_skipped(&mut self) {
        self.skipped += 1;
    }

    fn add_success(&mut self, size: Size) {
        self.success += 1;
        self.dl_size += size.bytes();
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", if self.success + self.skipped + self.failure > 0 {
            format!(
                "downloaded approx. {} for {} files / success: {} / skipped: {} / failure: {}",
                Size::from_bytes(self.dl_size),
                n_fmt(self.success + self.skipped + self.failure),
                n_fmt(self.success),
                n_fmt(self.skipped),
                n_fmt(self.failure)
            )
        } else {
            String::new()
        })
    }
}
