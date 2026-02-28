use clap::Parser;
use pretty_duration::pretty_duration;
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{ Display, Formatter, Result },
    num,
    sync::LazyLock,
    time::Duration,
};

pub static ARGUMENTS: LazyLock<Args> = LazyLock::new(Args::parse);

#[derive(Deserialize, Parser)]
#[clap(about, version, arg_required_else_help = true)]
pub struct Args {
    #[arg(help = "Creator page or post / Discord server or channel")]
    pub urls: Option<Vec<String>>,

    #[arg(short, long, help = "Proxy URL (scheme://host:port[/path])")]
    pub proxy: Option<String>,

    #[arg(short, long, default_value_t = 256, help = "Simultaneous downloads (1-512)")]
    threads: usize,

    #[arg(short = 'f', long = "input-file", help = "File with URLs, can be used multiple times")]
    pub input_files: Option<Vec<String>>,

    #[arg(short, long, default_value = "kumono", help = "Base directory for downloads")]
    pub output_path: String,

    #[arg(
        short,
        long,
        help = "List available file extensions (per URL)",
        help_heading = "Filtering"
    )]
    pub list_extensions: bool,

    #[arg(
        short,
        long,
        value_delimiter = ',',
        conflicts_with = "exclude",
        help = "File extensions to include (comma separated)",
        help_heading = "Filtering"
    )]
    include: Option<Vec<String>>,

    #[arg(
        short,
        long,
        value_delimiter = ',',
        conflicts_with = "include",
        help = "File extensions to exclude (comma separated)",
        help_heading = "Filtering"
    )]
    exclude: Option<Vec<String>>,

    #[arg(short, long, help = "Log hashes, skip moved/deleted file downloads")]
    pub download_archive: bool,

    #[arg(short, long, default_value_t = 4, help_heading = "Connection")]
    pub max_retries: usize,

    #[arg(
        short,
        long,
        value_parser = try_duration_from_secs,
        default_value = "1",
        help_heading = "Connection"
    )]
    pub retry_delay: Duration,

    #[arg(
        long,
        value_parser = try_duration_from_secs,
        default_value = "5",
        help_heading = "Connection"
    )]
    pub connect_timeout: Duration,

    // TODO: retry multiple times (or perhaps infinitely?) on timeout,
    // lower timeout (60~120s? measure average response time outlier to be sure)
    #[arg(
        long,
        value_parser = try_duration_from_secs,
        default_value = "180",
        help_heading = "Connection"
    )]
    pub read_timeout: Duration,

    #[arg(
        long,
        value_parser = try_duration_from_secs,
        default_value = "15",
        help_heading = "Connection"
    )]
    pub rate_limit_backoff: Duration,

    #[arg(
        long,
        value_parser = try_duration_from_secs,
        default_value = "5",
        help_heading = "Connection"
    )]
    pub server_error_delay: Duration,

    #[arg(short, long, help = "Print configuration values")]
    pub show_config: bool,
    // #[arg(short, long, help = "Print all error messages")]
    // pub verbose: bool,

    #[arg(short = 'C', long, requires = "coomer_pass", help_heading = "Login")]
    pub coomer_user: Option<String>,

    #[arg(short = 'c', long, requires = "coomer_user", help_heading = "Login")]
    pub coomer_pass: Option<String>,

    #[arg(short = 'K', long, requires = "kemono_pass", help_heading = "Login")]
    pub kemono_user: Option<String>,

    #[arg(short = 'k', long, requires = "kemono_user", help_heading = "Login")]
    pub kemono_pass: Option<String>,

    // #[arg(short = 'a', long = "cred_file", help = "Credential file")]
    // pub credential_path: Option<String>,

    // #[arg(short = 'J', long, help = "Cookies.txt location")]
    // pub cookie_jar: Option<String>,

    // #[arg(short = 'B, long = "pass", help = "Load cookies from browser")]
    // pub cookies_from_browser: Option<String>,
}

fn try_duration_from_secs(arg: &str) -> anyhow::Result<Duration, num::ParseIntError> {
    Ok(Duration::from_secs(arg.parse::<u64>()?.clamp(1, u64::MAX)))
}

impl Args {
    pub fn threads(&self) -> usize {
        self.threads.clamp(1, 512)
    }

    pub fn included(&self) -> Option<HashSet<String>> {
        Self::process_exts(self.include.as_ref()?)
    }

    pub fn excluded(&self) -> Option<HashSet<String>> {
        Self::process_exts(self.exclude.as_ref()?)
    }

    fn process_exts(exts: &[String]) -> Option<HashSet<String>> {
        let mut unique_exts = HashSet::with_capacity(exts.len());

        for ext in exts {
            if !unique_exts.contains(ext) {
                unique_exts.insert(ext.clone());
            }
        }

        if unique_exts.is_empty() {
            None
        } else {
            unique_exts.shrink_to_fit();
            Some(unique_exts)
        }
    }
}

impl Display for Args {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let pd = |d: &Duration| pretty_duration(d, None);

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
