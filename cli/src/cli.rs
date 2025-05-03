use kumono::Service;
use clap::{ arg, Parser };
use pretty_duration::pretty_duration;
use serde::Deserialize;
use std::{ fmt, num, sync::LazyLock, time::Duration };

pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

#[derive(Deserialize, Parser)]
pub struct Args {
    pub service: Service,

    #[arg(help = "Creator ID")]
    pub creator: String,

    #[arg(short, long, help = "SOCKS5 proxy (IP:Port)")]
    proxy: Option<String>,

    #[arg(short, long, default_value_t = 64)]
    pub threads: u8,

    #[arg(short, long, value_parser = duration_from_secs, default_value = "1")]
    pub connect_timeout: Duration,

    #[arg(short, long, value_parser = duration_from_secs, default_value = "5")]
    pub read_timeout: Duration,

    #[arg(short, long, value_parser = duration_from_secs, default_value = "45")]
    pub api_backoff: Duration,

    #[arg(short, long, value_parser = duration_from_secs, default_value = "15")]
    pub download_backoff: Duration,

    #[arg(short, long)]
    pub skip_initial_hash_verification: bool,
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
