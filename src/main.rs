use crate::{profile::Profile, stats::Stats};
use anyhow::Result;
use futures::future::join_all;
use log::error;
use std::{env, process, sync::Arc};
use tokio::{fs, sync::Semaphore, task};

mod client;
mod config;
mod profile;
mod stats;
mod usage;


#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<_> = env::args().filter(|arg| !arg.is_empty()).collect();

    if args.len() != 3 {
        usage::usage();
        process::exit(1);
    }

    let profile = Profile::new(&args[1], &args[2]).await?;

    fs::create_dir_all(&args[2]).await?;

    let mut tasks = vec![];

    let sem = Arc::new(Semaphore::new(config::CONCURRENCY));

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

    Ok(())
}
