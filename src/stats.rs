use crate::n_fmt;
use log::info;
use size::Size;

pub enum DownloadState {
    Fail(Size),
    Skip,
    Success(Size),
}

#[derive(Default)]
pub struct Stats {
    success: usize,
    skipped: usize,
    failure: usize,
    dl_size: i64,
}

impl Stats {
    #[allow(clippy::needless_pass_by_value)]
    pub fn update(&mut self, state: DownloadState) {
        match state {
            DownloadState::Fail(size) => self.add_failure(size),
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

    pub fn print(&self) {
        if self.success + self.skipped + self.failure > 0 {
            info!(
                "downloaded approx. {} for {} files / success: {} / skipped: {} / failure: {}",
                Size::from_bytes(self.dl_size),
                n_fmt(self.success + self.skipped + self.failure),
                n_fmt(self.success),
                n_fmt(self.skipped),
                n_fmt(self.failure)
            );
        }
    }
}
