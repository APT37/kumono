use clap::{ arg, Parser, ValueEnum };
use pretty_duration::pretty_duration;
use serde::Deserialize;
use std::{ fmt, num, sync::LazyLock, time::Duration };
use strum_macros::Display;

pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

#[derive(Deserialize, Parser)]
pub struct Args {
    #[arg(short, long)]
    pub service: Service,

    #[arg(short, long)]
    pub creator: String,

    #[arg(short, long)]
    proxy: Option<String>,

    #[arg(long)]
    pub skip_initial_hash_verification: bool,

    #[arg(default_value_t = 64)]
    pub threads: u8,

    #[arg(value_parser = duration_from_secs, default_value = "1")]
    pub connect_timeout: Duration,

    #[arg(value_parser = duration_from_secs, default_value = "5")]
    pub read_timeout: Duration,

    #[arg(value_parser = duration_from_secs, default_value = "45")]
    pub api_backoff: Duration,

    #[arg(value_parser = duration_from_secs, default_value = "15")]
    pub download_backoff: Duration,
}

fn duration_from_secs(arg: &str) -> Result<Duration, num::ParseIntError> {
    Ok(Duration::from_secs(arg.parse()?))
}

impl Args {
    pub fn proxy(&self) -> Option<String> {
        self.proxy.clone().map(|proxy| format!("socks5://{proxy}"))
    }
}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn pd(d: &Duration) -> String {
            pretty_duration(d, None)
        }

        write!(
            f,
            "Threads: {} / API Backoff: {} / Proxy: {} / Timeouts: (Connect: {} / Overall: {}) / Download Backoff: {}",
            self.threads,
            pd(&self.api_backoff),
            self.proxy.as_ref().unwrap_or(&String::from("None")),
            pd(&self.connect_timeout),
            pd(&self.read_timeout),
            pd(&self.download_backoff)
        )
    }
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Display, Clone, Copy, ValueEnum)]
pub enum Service {
    boosty,
    candfans,
    discord,
    dlsite,
    fanbox,
    fansly,
    fantia,
    gumroad,
    onlyfans,
    patreon,
    subscribestar,
}

impl Service {
    pub fn site(self) -> &'static str {
        use Service::{ candfans, fansly, onlyfans };

        match self {
            candfans | fansly | onlyfans => "coomer",
            _ => "kemono",
        }
    }
}
