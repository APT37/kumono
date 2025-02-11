use num_format::{Locale, ToFormattedString};
use size::Size;

pub struct Stats {
    success: usize,
    skipped: usize,
    failure: usize,

    dl_size: i64,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            success: 0,
            skipped: 0,
            failure: 0,

            dl_size: 0,
        }
    }

    pub fn add_success(&mut self) {
        self.success += 1;
    }

    pub fn add_skipped(&mut self) {
        self.skipped += 1;
    }

    pub fn add_failure(&mut self) {
        self.failure += 1;
    }

    pub fn add_size(&mut self, size: Size) {
        self.dl_size += size.bytes();
    }

    pub fn print(&self) {
        if self.success + self.skipped + self.failure > 0 {
            log::info!(
                "downloaded approx. {} / total: {} / success: {} / skipped: {} / failure: {}",
                Size::from_bytes(self.dl_size),
                (self.success + self.failure).to_formatted_string(&Locale::de),
                self.success.to_formatted_string(&Locale::de),
                self.skipped.to_formatted_string(&Locale::de),
                self.failure.to_formatted_string(&Locale::de),
            );
        }
    }
}
