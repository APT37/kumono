use crate::{ cli::ARGS, profile::Profile, progress::DownloadState, target::TARGETS };
use anyhow::Result;
use futures::future::join_all;
use std::{ collections::HashSet, path::PathBuf, sync::Arc, thread };
use tokio::{ fs, sync::{ Semaphore, mpsc }, task, time::{ Duration, sleep } };

mod api;
mod cli;
mod file;
mod http;
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

    for (i, target) in TARGETS.clone().into_iter().enumerate() {
        let mut files = Profile::new(&target).await?.files;

        if files.is_empty() {
            continue;
        }

        if ARGS.list_extensions {
            let mut extensions = HashSet::new();
            let mut no_ext = 0;

            for file in files {
                if let Some(ext) = file.to_extension(&target) {
                    extensions.insert(ext.to_lowercase());
                } else {
                    no_ext += 1;
                }
            }

            if no_ext > 0 {
                eprintln!("{no_ext} files do not have an extension");
            }

            if !extensions.is_empty() {
                eprintln!("{}", extensions.into_iter().collect::<Vec<_>>().join(","));

                if i != TARGETS.len() - 1 {
                    eprintln!();
                }
            }
        } else {
            if let Some(exts) = ARGS.included() {
                files.retain(|file| {
                    file.to_extension(&target).is_some() &&
                        exts.contains(&file.to_extension(&target).unwrap().to_lowercase())
                });
            } else if let Some(exts) = ARGS.excluded() {
                files.retain(|file| {
                    file.to_extension(&target).is_none() ||
                        !exts.contains(&file.to_extension(&target).unwrap().to_lowercase())
                });
            }

            let len = files.len();

            if len == 0 {
                eprintln!(
                    "No files match the current extension filters.\nPlease use '--list' to view available extensions."
                );
                return Ok(());
            }

            fs::create_dir_all(target.to_pathbuf(None)).await?;

            let (tx, rx) = mpsc::channel::<DownloadState>(len);

            let archive_path = target.to_archive_pathbuf();

            thread::spawn(move || progress::bar(rx, archive_path, len as u64));

            let mut tasks = Vec::new();

            let sem = Arc::new(Semaphore::new(ARGS.threads()));

            for file in files {
                let permit = sem.clone().acquire_owned().await;

                let tx = tx.clone();

                let target = target.clone();

                tasks.push(
                    task::spawn(async move {
                        #[allow(clippy::no_effect_underscore_binding)]
                        let _permit = permit;

                        let result = file.download(&target).await;

                        match result {
                            Ok(dl_state) => {
                                tx.send(dl_state).await.expect("send state to progress bar");
                            }
                            Err(err) => {
                                let mut error = err.to_string();
                                if let Some(source) = err.source() {
                                    error.push('\n');
                                    error.push_str(&source.to_string());
                                }
                                tx.send(DownloadState::Failure(u64::default(), error)).await.expect(
                                    "send state to progress bar"
                                );
                            }
                        }
                    })
                );
            }

            join_all(tasks).await;

            // wait so the bar can finish properly
            sleep(Duration::from_millis(1)).await;
        }
    }

    Ok(())
}
