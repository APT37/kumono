use crate::services::Service;
use clap::{ Parser, arg };
use pretty_duration::pretty_duration;
use serde::Deserialize;
use std::{ fmt, net::SocketAddr, num, path::PathBuf, sync::LazyLock, time::Duration };

pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

#[derive(Deserialize, Parser)]
#[clap(about, version)]
pub struct Args {
    pub service: Service,

    pub user_id: String,

    #[arg(short, long, help = "SOCKS5 proxy (IP:Port)")]
    pub proxy: Option<SocketAddr>,

    #[arg(short, long, default_value_t = 256, help = "Simultaneous downloads")]
    pub threads: u16,

    #[arg(long, value_parser = duration_from_secs, default_value = "1")]
    pub connect_timeout: Duration,

    #[arg(long, value_parser = duration_from_secs, default_value = "5")]
    pub read_timeout: Duration,

    #[arg(long, value_parser = duration_from_secs, default_value = "15")]
    pub rate_limit_backoff: Duration,

    #[arg(long, value_parser = duration_from_secs, default_value = "5")]
    pub server_error_delay: Duration,
}

fn duration_from_secs(arg: &str) -> Result<Duration, num::ParseIntError> {
    Ok(Duration::from_secs(arg.parse()?))
}

impl Args {
    pub fn to_pathbuf(&self) -> PathBuf {
        PathBuf::from_iter([&self.service.to_string(), &self.user_id])
    }

    pub fn to_pathbuf_with_file(&self, file: impl AsRef<str>) -> PathBuf {
        PathBuf::from_iter([&self.service.to_string(), &self.user_id, file.as_ref()])
    }
}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn pd(d: &Duration) -> String {
            pretty_duration(d, None)
        }

        write!(
            f,
            "Threads: {} / Proxy: {} / Timeout: (Connect: {} / Read: {}) / Backoff: (Rate Limit: {} / Server Error: {})",
            self.threads,
            self.proxy.map_or(String::from("None"), |p| p.to_string()),
            pd(&self.connect_timeout),
            pd(&self.read_timeout),
            pd(&self.rate_limit_backoff),
            pd(&self.server_error_delay)
        )
    }
}
