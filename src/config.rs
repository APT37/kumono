use anyhow::Result;
use log::{ error, info };
use pretty_duration::pretty_duration;
use serde::Deserialize;
use std::{ env, fmt, fs, path::PathBuf, process, sync::LazyLock, time::Duration };

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config::parse().unwrap_or_else(|err| {
        error!("{err}");
        process::exit(1);
    })
});

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default = "concurrency")]
    pub concurrency: u8,

    #[serde(default = "connect_timeout")]
    pub connect_timeout: Duration,
    #[serde(default = "read_timeout")]
    pub read_timeout: Duration,

    #[serde(default = "api_delay_ms")]
    pub api_delay_ms: Duration,
    #[serde(default = "api_backoff")]
    pub api_backoff: Duration,

    #[serde(default = "download_backoff")]
    pub download_backoff: Duration,

    pub proxy: Option<String>,
}

fn concurrency() -> u8 {
    64
}

fn connect_timeout() -> Duration {
    Duration::from_secs(1)
}

fn read_timeout() -> Duration {
    Duration::from_secs(5)
}

fn api_delay_ms() -> Duration {
    Duration::from_millis(100)
}

fn api_backoff() -> Duration {
    Duration::from_secs(45)
}

fn download_backoff() -> Duration {
    Duration::from_secs(15)
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

    pub fn proxy(&self) -> Option<String> {
        self.proxy.clone().map(|proxy| format!("socks5://{proxy}"))
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn pd(d: &Duration) -> String {
            pretty_duration(d, None)
        }

        write!(
            f,
            "Concurrency: {} / API: (Delay: {} / Backoff: {}) / Proxy: {} / Timeouts: (Connect: {} / Overall: {}) / Download Backoff: {}",
            self.concurrency,
            pd(&self.api_delay_ms),
            pd(&self.api_backoff),
            self.proxy.as_ref().unwrap_or(&String::from("None")),
            pd(&self.connect_timeout),
            pd(&self.read_timeout),
            pd(&self.download_backoff)
        )
    }
}
