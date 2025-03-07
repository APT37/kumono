use crate::{config::CONFIG, profile::Profile, stats::Stats};
use anyhow::Result;
use futures::future::join_all;
use log::{error, info};
use std::sync::Arc;
use tokio::{fs, sync::Semaphore, task};

mod client;
mod config;
mod input;
mod profile;
mod stats;
mod usage;

#[tokio::main]
async fn main() -> Result<()> {
    colog::init();

    let args = input::args();

    info!("{}", *CONFIG);

    let profile = Profile::new(&args[1], &args[2]).await?;

    fs::create_dir_all(&args[2]).await?;

    let mut tasks = vec![];

    let sem = Arc::new(Semaphore::new(CONFIG.concurrency()));

    for file in profile.files {
        let permit = sem.clone().acquire_owned().await;

        tasks.push(task::spawn(async move {
            // aww, the compiler thinks this is useless :)
            #[allow(clippy::no_effect_underscore_binding)]
            let _permit = permit;

            file.download().await
        }));
    }

    let mut stats = Stats::new();

    for task in join_all(tasks).await {
        match task? {
            Ok((true, size)) => {
                stats.add_success();
                stats.add_size(size);
            }
            Ok((false, _)) => stats.add_skipped(),
            Err(err) => {
                stats.add_failure();
                error!("{err}");
            }
        }
    }

    stats.print();

    let _ = fs::remove_dir(&args[2]).await;

    Ok(())
}
