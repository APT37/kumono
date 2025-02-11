use std::time::Duration;

pub const CONCURRENCY: usize = 32;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

pub const API_DELAY: Duration = Duration::from_millis(100);
pub const API_BACKOFF: Duration = Duration::from_secs(45);

pub const PROXY: &str = "socks5://10.124.0.4:1080";
