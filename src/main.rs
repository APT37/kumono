use crate::{ config::CONFIG, input::ARGS, profile::Profile, stats::{ DownloadState, Stats } };
use anyhow::Result;
use futures::future::join_all;
use log::{ error, info };
use num_format::{ Locale, ToFormattedString };
use size::Size;
use std::{ process, sync::Arc };
use tokio::{ fs, sync::Semaphore, task };

mod client;
mod config;
mod input;
mod profile;
mod stats;

fn n_fmt(n: usize) -> String {
    n.to_formatted_string(&Locale::de)
}

#[tokio::main]
async fn main() -> Result<()> {
    colog::init();

    info!("{}", *CONFIG);

    let profile = Profile::new(ARGS.service, &ARGS.creator).await?;

    if profile.files.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(&ARGS.creator).await?;

    let mut tasks = vec![];

    let sem = Arc::new(Semaphore::new(CONFIG.concurrency.into()));

    for file in profile.files {
        let permit = sem.clone().acquire_owned().await;

        tasks.push(
            task::spawn(async move {
                // aww, the compiler thinks this is useless :)
                #[allow(clippy::no_effect_underscore_binding)]
                let _permit = permit;

                file.download(ARGS.service, &ARGS.creator).await
            })
        );
    }

    let mut stats = Stats::default();

    for task in join_all(tasks).await {
        match task? {
            Ok(dl_state) => stats.update(dl_state),
            Err(err) => {
                error!("{err}{}", if let Some(s) = err.source() {
                    format!("\n{s}")
                } else {
                    String::new()
                });
                stats.update(DownloadState::Failure(Size::default()));
            }
        }
    }

    stats.print();

    let _ = fs::remove_dir(&ARGS.creator).await;

    if stats.failure != 0 {
        process::exit(1);
    }

    Ok(())
}
