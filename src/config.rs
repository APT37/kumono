use lazy_static::lazy_static;
use log::error;
use pretty_duration::pretty_duration;
use serde::Deserialize;
use std::{env, fmt, fs, process, time::Duration};

lazy_static! {
    pub static ref CONFIG: Config = Config::parse();
}

const DEFAULT_CONCURRENCY: usize = 32;
const MAX_CONCURRENCY: usize = 256;

const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 1000;
const DEFAULT_TIMEOUT_MS: u64 = 5000;

const DEFAULT_API_DELAY_MS: u64 = 100;
const DEFAULT_API_BACKOFF: u64 = 45;

#[derive(Deserialize)]
pub struct Config {
    concurrency: Option<usize>,

    connect_timeout_ms: Option<u64>,
    read_timeout_ms: Option<u64>,

    api_delay_ms: Option<u64>,
    api_backoff: Option<u64>,

    proxy: Option<String>,
}

impl Config {
    fn parse() -> Self {
        let home = env::var("HOME").unwrap_or_else(|err| {
            error!("{err}");
            process::exit(1);
        });

        toml::from_str::<Config>(
            &fs::read_to_string(format!("{home}/.config/coomer-rip.toml")).unwrap_or_else(|err| {
                error!("{err}");
                process::exit(1);
            }),
        )
        .unwrap_or_else(|err| {
            error!("{err}");
            process::exit(1);
        })
    }

    pub fn concurrency(&self) -> usize {
        self.concurrency
            .unwrap_or(DEFAULT_CONCURRENCY)
            .clamp(1, MAX_CONCURRENCY)
    }

    pub fn connect_timeout(&self) -> Duration {
        Duration::from_millis(
            self.connect_timeout_ms
                .unwrap_or(DEFAULT_CONNECT_TIMEOUT_MS),
        )
    }

    pub fn read_timeout(&self) -> Duration {
        Duration::from_millis(self.read_timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS))
    }

    pub fn api_delay(&self) -> Duration {
        Duration::from_millis(self.api_delay_ms.unwrap_or(DEFAULT_API_DELAY_MS))
    }

    pub fn api_backoff(&self) -> Duration {
        Duration::from_secs(self.api_backoff.unwrap_or(DEFAULT_API_BACKOFF))
    }

    pub fn proxy(&self) -> Option<String> {
        self.proxy.clone().map(|p| format!("socks5://{p}"))
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn pd(d: &Duration) -> String {
            pretty_duration(d, None)
        }

        write!(
            f,
            "Concurrent Downloads: {} / Delay: {} / Backoff: {} / Proxy: {} / Connect Timeout: {} / Timeout: {}",
            self.concurrency(),
            pd(&self.api_delay()),
            pd(&self.api_backoff()),
            self.proxy.as_ref().unwrap_or(&"None".to_string()),
            pd(&self.connect_timeout()),
            pd(&self.read_timeout()),
        )
    }
}
