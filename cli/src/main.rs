use crate::{ cli::ARGS, profile::Profile, progress::DownloadState };
use anyhow::Result;
use futures::future::join_all;
use size::Size;
use std::{ path::PathBuf, sync::Arc, thread };
use tokio::{ fs, sync::{ mpsc, Semaphore }, task };

mod cli;
mod http;
mod profile;
mod progress;

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("{}", *ARGS);

    let profile = Profile::new(ARGS.service, &ARGS.creator).await?;

    if profile.files.is_empty() {
        return Ok(());
    }

    let path = PathBuf::from_iter([&ARGS.service.to_string(), &ARGS.creator]);

    let len = profile.files.len();

    if len > 0 {
        fs::create_dir_all(&path).await?;

        let (tx, rx) = mpsc::channel::<DownloadState>(len);

        thread::spawn(move || progress::bar(rx, len as u64));

        let mut tasks = vec![];

        let sem = Arc::new(Semaphore::new(ARGS.threads.into()));

        for file in profile.files {
            let permit = sem.clone().acquire_owned().await;

            let tx = tx.clone();

            tasks.push(
                task::spawn(async move {
                    // aww, the compiler thinks this is useless :)
                    #[allow(clippy::no_effect_underscore_binding)]
                    let _permit = permit;

                    let result = file.download(ARGS.service, &ARGS.creator).await;

                    match result {
                        Ok(dl_state) => {
                            tx.send(dl_state).await.expect("send state to progress bar");
                        }
                        Err(error) => {
                            let prefix = format!(
                                "{error}{}\n",
                                error.source().map_or_else(String::new, |s| format!("\n{s}"))
                            );
                            tx.send(DownloadState::Failure(Size::default(), prefix)).await.expect(
                                "send state to progress bar"
                            );
                        }
                    }
                })
            );
        }

        join_all(tasks).await;
    }

    #[allow(unused_must_use)]
    fs::remove_dir(&path).await;

    Ok(())
}
