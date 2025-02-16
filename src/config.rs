use anyhow::Result;
use log::{error, info};
use pretty_duration::pretty_duration;
use serde::Deserialize;
use std::{env, fmt, fs, path::PathBuf, process, sync::LazyLock, time::Duration};

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config::parse().unwrap_or_else(|err| {
        error!("{err}");
        process::exit(1);
    })
});

const DEFAULT_CONCURRENCY: usize = 32;
const MAX_CONCURRENCY: usize = 256;

const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 1000;
const DEFAULT_TIMEOUT_MS: u64 = 5000;

const DEFAULT_API_DELAY_MS: u64 = 100;
const DEFAULT_API_BACKOFF: u64 = 45;

#[derive(Deserialize, Default)]
pub struct Config {
    concurrency: Option<usize>,

    connect_timeout_ms: Option<u64>,
    read_timeout_ms: Option<u64>,

    api_delay_ms: Option<u64>,
    api_backoff: Option<u64>,

    proxy: Option<String>,
}

impl Config {
    fn parse() -> Result<Self> {
        let home = env::var("HOME")?;

        let path = PathBuf::from_iter([&home, ".config", "coomer-rip.toml"]);

        let config = if path.try_exists()? {
            info!("using configuration file {}", path.to_string_lossy());

            let file = fs::read_to_string(path)?;

            toml::from_str::<Config>(&file)?
        } else {
            Config::default()
        };

        Ok(config)
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
