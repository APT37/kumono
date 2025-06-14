use clap::{ Parser, arg };
use pretty_duration::pretty_duration;
use serde::{ Deserialize };
use std::{ fmt, net::SocketAddr, num, sync::LazyLock, time::Duration };

pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

#[derive(Deserialize, Parser)]
#[clap(about, version, arg_required_else_help = true)]
pub struct Args {
    #[arg(help = "Creator page or post / Discord server or channel)")]
    pub url: String,

    #[arg(short, long, help = "SOCKS5 proxy (IP:Port)")]
    pub proxy: Option<SocketAddr>,

    #[arg(short, long, default_value_t = 256, help = "Simultaneous downloads")]
    pub threads: u16,

    #[arg(
        short,
        long,
        num_args = 1..,
        value_delimiter = ',',
        conflicts_with = "exclude",
        help = "File extensions to include (comma separated)"
    )]
    include: Option<Vec<String>>,

    #[arg(
        short,
        long,
        num_args = 1..,
        value_delimiter = ',',
        conflicts_with = "include",
        help = "File extensions to exclude (comma separated)"
    )]
    exclude: Option<Vec<String>>,

    #[arg(short, long, help = "List of available file extensions (per target)")]
    pub list_extensions: bool,

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
    pub fn included(&self) -> Option<Vec<String>> {
        if let Some(exts) = &self.include {
            let mut exts: Vec<String> = exts
                .iter()
                .map(|ext| ext.to_lowercase())
                .collect();

            exts.sort();
            exts.dedup();

            Some(exts)
        } else {
            None
        }
    }

    pub fn excluded(&self) -> Option<Vec<String>> {
        if let Some(exts) = &self.exclude {
            let mut exts: Vec<String> = exts
                .iter()
                .map(|ext| ext.to_lowercase())
                .collect();

            exts.sort();
            exts.dedup();

            Some(exts)
        } else {
            None
        }
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
