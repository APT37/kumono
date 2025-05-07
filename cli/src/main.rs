use crate::{ cli::ARGS, profile::Profile, progress::DownloadState };
use anyhow::Result;
use futures::future::join_all;
use size::Size;
use std::{ path::PathBuf, sync::Arc, thread };
use tokio::{ fs, sync::{ mpsc, Semaphore }, task, time::{ Duration, sleep } };

mod cli;
mod http;
mod profile;
mod progress;

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("{}", *ARGS);

    let profile = Profile::new(ARGS.service, &ARGS.user_id).await?;

    if profile.files.is_empty() {
        return Ok(());
    }

    let path = PathBuf::from_iter([&ARGS.service.to_string(), &ARGS.user_id]);

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
                    #[allow(clippy::no_effect_underscore_binding)]
                    let _permit = permit;

                    let result = file.download(ARGS.service, &ARGS.user_id).await;

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
                            tx.send(DownloadState::Failure(Size::default(), error)).await.expect(
                                "send state to progress bar"
                            );
                        }
                    }
                })
            );
        }

        join_all(tasks).await;
    }

    // wait as bit so the bar can finish properly
    sleep(Duration::from_millis(1)).await;

    #[allow(unused_must_use)]
    fs::remove_dir(&path).await;

    Ok(())
}
