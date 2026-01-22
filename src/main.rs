use crate::{ cli::ARGUMENTS, profile::Profile, progress::DownloadAction, target::Target };
use anyhow::Result;
use futures::future::join_all;
use itertools::Itertools;
use std::{ path::PathBuf, process::exit, sync::{ Arc, atomic::Ordering::Relaxed }, thread };
use strum_macros::Display;
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

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() -> Result<()> {
    if ARGUMENTS.show_config {
        eprintln!("{}", *ARGUMENTS);
    }

    if ARGUMENTS.download_archive {
        fs::create_dir_all(PathBuf::from_iter([&ARGUMENTS.output_path, "db"])).await?;
    }

    let mut targets = Target::parse_args().await;

    targets.append(&mut Target::try_parse_file().await?);

    targets = targets.into_iter().unique().collect();

    if targets.is_empty() {
        eprintln!("No valid target URLs.");
        exit(3);
    }

    let total_targets = targets.len();

    for (i, target) in targets.into_iter().enumerate() {
        let mut files = Profile::try_new(&target, i + 1).await?.files;

        if files.is_empty() {
            if i != total_targets - 1 {
                eprintln!();
            }
            continue;
        }

        if ARGUMENTS.list_extensions {
            eprintln!("{}", ext::ExtensionList::new(&files, &target));

            if i != total_targets - 1 {
                eprintln!();
            }
            continue;
        }

        let mut total = files.len();

        if let Some(exts) = ARGUMENTS.included() {
            files.retain(|file| {
                file.to_extension(&target).is_some_and(|ext| exts.contains(&ext))
            });
            files_left_msg(Filter::Inclusive, total, files.len());
        } else if let Some(exts) = ARGUMENTS.excluded() {
            files.retain(|file| {
                file.to_extension(&target).is_none_or(|ext| !exts.contains(&ext))
            });
            files_left_msg(Filter::Exclusive, total, files.len());
        }

        if files.is_empty() {
            if i != total_targets - 1 {
                eprintln!();
            }
            continue;
        }

        if ARGUMENTS.download_archive {
            total = files.len();

            let archive = target.archive();

            files.retain(|file| file.to_hash().is_none_or(|hash| !archive.contains(&hash)));

            let left = files.len();

            if total != left {
                files_left_msg(Filter::DownloadArchive, total, left);
            }
        }

        if files.is_empty() {
            if i != total_targets - 1 {
                eprintln!();
            }
            continue;
        }

        let left = files.len();

        fs::create_dir_all(target.to_pathbuf(None)).await?;

        let archive_path = target.to_archive_pathbuf();

        let (msg_tx, msg_rx) = mpsc::channel::<DownloadAction>(left);

        let files_by_type = ext::count(&files, &target);

        thread::spawn(move || {
            progress::progress_bar(
                left as u64,
                archive_path,
                msg_rx,
                i == total_targets - 1,
                files_by_type
            );
        });

        let mut tasks = Vec::with_capacity(files.len());

        let sem = Arc::new(Semaphore::new(ARGUMENTS.threads()));

        for file in files {
            let permit = sem.clone().acquire_owned().await;
            let msg_tx = msg_tx.clone();
            let target = target.clone();

            tasks.push(
                task::spawn(async move {
                    let _permit = permit;

                    match file.try_download(&target, msg_tx.clone()).await {
                        Ok(action) => msg_tx.send(action).await.unwrap(),
                        Err(err) => {
                            let mut error = err.to_string();

                            if let Some(source) = err.source() {
                                error.push('\n');
                                error.push_str(&source.to_string());
                            }

                            msg_tx
                                .send(DownloadAction::Fail(error, file.to_extension(&target))).await
                                .unwrap();
                        }
                    }
                })
            );
        }

        join_all(tasks).await;

        // wait for the bar to (hopefully) finish properly
        sleep(Duration::from_millis((left / 10).try_into().unwrap_or_default())).await;
    }

    if progress::DOWNLOADS_FAILED.load(Relaxed) {
        exit(4);
    }

    Ok(())
}

fn files_left_msg(filter: Filter, total: usize, left: usize) {
    eprintln!(
        "{filter}: skipping {skipped}, {left} left to download/check",
        skipped = pretty::with_noun((total - left) as u64, "file"),
        left = pretty::with_noun(left as u64, "file")
    );
}

#[derive(Debug, Clone, Copy, Display)]
enum Filter {
    Inclusive,
    Exclusive,
    DownloadArchive,
}
