use crate::{ cli::ARGS, profile::Profile, progress::DownloadAction, target::Target };
use anyhow::Result;
use futures::future::join_all;
use strum_macros::Display;
use std::{ path::PathBuf, process::exit, sync::Arc, thread };
use tokio::{ fs, sync::{ Semaphore, mpsc }, task, time::{ Duration, sleep } };

mod api;
mod cli;
mod ext;
mod file;
mod http;
mod pretty;
mod profile;
mod progress;
mod target;

#[tokio::main]
async fn main() -> Result<()> {
    if ARGS.show_config {
        eprintln!("{}", *ARGS);
    }

    if ARGS.download_archive {
        fs::create_dir_all(PathBuf::from_iter([&ARGS.output_path, "db"])).await?;
    }

    let targets = Target::from_args().await;

    let total_targets = targets.len();

    for (i, target) in targets.into_iter().enumerate() {
        let mut files = Profile::new(&target, i + 1).await?.files;

        if files.is_empty() {
            if i != total_targets - 1 {
                eprintln!();
            }
            continue;
        }

        if ARGS.list_extensions {
            ext::list(files, &target);

            if i != total_targets - 1 {
                eprintln!();
            }

            continue;
        }

        let mut total = files.len();

        if let Some(exts) = ARGS.included() {
            files.retain(|file| {
                file.to_extension(&target).is_some() &&
                    exts.contains(&file.to_extension(&target).unwrap().to_lowercase())
            });

            files_left_msg(Filter::Inclusive, total, files.len());
        } else if let Some(exts) = ARGS.excluded() {
            files.retain(|file| {
                file.to_extension(&target).is_none() ||
                    !exts.contains(&file.to_extension(&target).unwrap().to_lowercase())
            });

            files_left_msg(Filter::Exclusive, total, files.len());
        }

        if files.is_empty() {
            if i != total_targets - 1 {
                eprintln!();
            }
            continue;
        }

        if ARGS.download_archive {
            total = files.len();

            let archive = target.archive();

            files.retain(|f| {
                if let Some(hash) = f.to_hash() { !archive.contains(&hash) } else { true }
            });

            let left = files.len();

            if total != left {
                files_left_msg(Filter::DownloadArchive, total, left);
            }
        }

        let left = files.len();

        fs::create_dir_all(target.to_pathbuf(None)).await?;

        let archive_path = target.to_archive_pathbuf();

        let (msg_tx, msg_rx) = mpsc::channel::<DownloadAction>(left);

        thread::spawn(move ||
            progress::bar(left as u64, archive_path, msg_rx, i == total_targets - 1)
        );

        let mut tasks = Vec::new();

        let sem = Arc::new(Semaphore::new(ARGS.threads()));

        for file in files {
            let permit = sem.clone().acquire_owned().await;

            let msg_tx = msg_tx.clone();

            let target = target.clone();

            tasks.push(
                task::spawn(async move {
                    #[allow(clippy::no_effect_underscore_binding)]
                    let _permit = permit;

                    let result = file.download(&target, msg_tx.clone()).await;

                    match result {
                        Ok(action) => {
                            msg_tx.send(action).await.expect("send state to progress bar");
                        }
                        Err(err) => {
                            let mut error = err.to_string();
                            if let Some(source) = err.source() {
                                error.push('\n');
                                error.push_str(&source.to_string());
                            }
                            msg_tx
                                .send(DownloadAction::Fail(error)).await
                                .expect("send state to progress bar");
                        }
                    }
                })
            );
        }

        join_all(tasks).await;

        // wait so the bar can finish properly
        sleep(Duration::from_millis((left / 10).try_into().unwrap_or_default())).await;
    }

    if progress::downloads_failed() {
        exit(1);
    }

    Ok(())
}

fn files_left_msg(filter: Filter, total: usize, left: usize) {
    eprintln!(
        "{filter}: skipping {}, {} left to download/check",
        pretty::files(total - left),
        pretty::files(left)
    );
}

#[derive(Debug, Clone, Copy, Display)]
enum Filter {
    Inclusive,
    Exclusive,
    DownloadArchive,
}
