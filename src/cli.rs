use anyhow::Result;
use clap::{ Parser, arg };
use itertools::Itertools;
use pretty_duration::pretty_duration;
use serde::Deserialize;
use std::{ fmt, num, sync::LazyLock, time::Duration };

pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

#[derive(Deserialize, Parser)]
#[clap(about, version, arg_required_else_help = true)]
pub struct Args {
    #[arg(help = "Creator page or post / Discord server or channel")]
    pub urls: Vec<String>,

    #[arg(short, long, help = "Proxy URL (scheme://host:port[/path])")]
    pub proxy: Option<String>,

    #[arg(short, long, default_value_t = 256, help = "Simultaneous downloads (1-4096)")]
    threads: usize,

    #[arg(short, long, default_value = "kumono", help = "Base directory for downloads")]
    pub output_path: String,

    #[arg(short, long, help = "List of available file extensions (per target)")]
    pub list_extensions: bool,

    #[arg(
        short,
        long,
        value_delimiter = ',',
        conflicts_with = "exclude",
        help = "File extensions to include (comma separated)"
    )]
    include: Option<Vec<String>>,

    #[arg(
        short,
        long,
        value_delimiter = ',',
        conflicts_with = "include",
        help = "File extensions to exclude (comma separated)"
    )]
    exclude: Option<Vec<String>>,

    #[arg(short, long, help = "Store hashes to skip previously downloaded files")]
    pub download_archive: bool,

    #[arg(short, long, default_value = "5")]
    pub max_retries: usize,

    #[arg(short, long, value_parser = duration_from_secs, default_value = "1")]
    pub retry_delay: Duration,

    #[arg(long, value_parser = duration_from_secs, default_value = "1")]
    pub connect_timeout: Duration,

    #[arg(long, value_parser = duration_from_secs, default_value = "5")]
    pub read_timeout: Duration,

    #[arg(long, value_parser = duration_from_secs, default_value = "15")]
    pub rate_limit_backoff: Duration,

    #[arg(long, value_parser = duration_from_secs, default_value = "5")]
    pub server_error_delay: Duration,

    #[arg(short, long, help = "Print configuration before execution")]
    pub show_config: bool,

    // #[arg(short, long, help = "Print verbose output")]
    // pub verbose: bool,
}

fn duration_from_secs(arg: &str) -> Result<Duration, num::ParseIntError> {
    Ok(Duration::from_secs(arg.parse::<u64>()?.clamp(1, u64::MAX)))
}

impl Args {
    pub fn threads(&self) -> usize {
        self.threads.clamp(1, 4096)
    }

    pub fn included(&self) -> Option<Vec<String>> {
        Self::process_exts(self.exclude.as_ref()?)
    }

    pub fn excluded(&self) -> Option<Vec<String>> {
        Self::process_exts(self.exclude.as_ref()?)
    }

    fn process_exts(exts: &[String]) -> Option<Vec<String>> {
        let exts: Vec<String> = exts
            .iter()
            .unique()
            .map(|ext| ext.to_lowercase())
            .collect();

        if exts.is_empty() {
            None
        } else {
            Some(exts)
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
            self.threads(),
            self.proxy.as_ref().map_or("None", |p| p),
            pd(&self.connect_timeout),
            pd(&self.read_timeout),
            pd(&self.rate_limit_backoff),
            pd(&self.server_error_delay)
        )
    }
}
