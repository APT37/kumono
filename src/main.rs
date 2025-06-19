use crate::{ cli::ARGS, profile::Profile, progress::DownloadState, target::TARGET };
use anyhow::Result;
use futures::future::join_all;
use std::{ collections::HashSet, sync::Arc, thread };
use tokio::{ fs, sync::{ Semaphore, mpsc }, task, time::{ Duration, sleep } };

mod cli;
mod http;
mod profile;
mod progress;
mod target;

#[tokio::main]
async fn main() -> Result<()> {
    if ARGS.show_config {
        eprintln!("{}", *ARGS);
    }

    let mut files = Profile::init().await?.files;

    if files.is_empty() {
        return Ok(());
    }

    if ARGS.list_extensions {
        let mut extensions = HashSet::new();
        let mut no_ext = 0;

        for file in files {
            if let Some(ext) = file.to_extension() {
                extensions.insert(ext.to_lowercase());
            } else {
                no_ext += 1;
            }
        }

        if no_ext > 0 {
            eprintln!("{no_ext} files do not have an extension");
        }

        if !extensions.is_empty() {
            eprintln!();
            println!("{}", extensions.into_iter().collect::<Vec<_>>().join(","));
        }
    } else {
        if let Some(exts) = ARGS.included() {
            files.retain(
                |file|
                    file.to_extension().is_some() &&
                    exts.contains(&file.to_extension().unwrap().to_lowercase())
            );
        } else if let Some(exts) = ARGS.excluded() {
            files.retain(
                |file|
                    file.to_extension().is_none() ||
                    !exts.contains(&file.to_extension().unwrap().to_lowercase())
            );
        }

        let len = files.len();

        if len == 0 {
            eprintln!("No files match the current extension filters.\nPlease use '--list' to view available extensions.");
            return Ok(());
        }

        fs::create_dir_all(TARGET.to_pathbuf(None)).await?;

        let (tx, rx) = mpsc::channel::<DownloadState>(len);

        thread::spawn(move || progress::bar(rx, len as u64));

        let mut tasks = vec![];

        let sem = Arc::new(Semaphore::new(ARGS.threads.into()));

        for file in files {
            let permit = sem.clone().acquire_owned().await;

            let tx = tx.clone();

            tasks.push(
                task::spawn(async move {
                    #[allow(clippy::no_effect_underscore_binding)]
                    let _permit = permit;

                    let result = file.download().await;

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

        #[allow(unused_must_use)]
        fs::remove_dir(TARGET.to_pathbuf(None)).await;
    }

    Ok(())
}
