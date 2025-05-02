use crate::{ cli::ARGS, profile::Profile, stats::{ DownloadState, Stats } };
use anyhow::Result;
use futures::future::join_all;
use indicatif::{ ProgressBar, ProgressStyle };
use num_format::{ Locale, ToFormattedString };
use size::Size;
use std::{ path::PathBuf, process, sync::Arc, thread };
use tokio::{ fs, sync::{ mpsc, Semaphore }, task };

mod client;
mod cli;
mod profile;
mod stats;

enum Message {
    Download(String, Option<String>),
    Stats(Stats),
}

fn n_fmt(n: usize) -> String {
    n.to_formatted_string(&Locale::de)
}

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("{}", *ARGS);

    let profile = Profile::new(ARGS.service, &ARGS.creator).await?;

    if profile.files.is_empty() {
        return Ok(());
    }

    let path = PathBuf::from_iter([&ARGS.service.to_string(), &ARGS.creator]);

    fs::create_dir_all(&path).await?;

    let len = profile.files.len();

    let (tx, rx) = mpsc::channel::<Message>(len);

    thread::spawn(move || progress_bar(rx, len as u64));

    let tx = Arc::new(tx);

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

                let name = file.to_name(ARGS.service, &ARGS.creator);

                match result {
                    Ok(dl_state) => {
                        tx.send(Message::Download(name, None)).await.expect(
                            "send name to progress bar"
                        );
                        Ok(dl_state)
                    }
                    Err(error) => {
                        let prefix = format!("{error}{}\n", if let Some(s) = error.source() {
                            format!("\n{s}")
                        } else {
                            String::new()
                        });

                        tx.send(Message::Download(name, Some(prefix))).await.expect(
                            "send name to progress bar"
                        );
                        Err(error)
                    }
                }
            })
        );
    }

    let mut stats = Stats::default();

    for task in join_all(tasks).await {
        match task? {
            Ok(dl_state) =>
                match dl_state {
                    DownloadState::Failure(size, err) => {
                        if let Some(error) = err {
                            tx.send(Message::Download(String::new(), Some(error))).await?;
                        }

                        stats.update(DownloadState::Failure(size, None));
                    }

                    dl_state => stats.update(dl_state),
                }

            Err(_) => stats.update(DownloadState::Failure(Size::default(), None)),
        }
    }

    tx.send(Message::Stats(stats)).await?;

    let _ = fs::remove_dir(&path).await;

    if stats.failure != 0 {
        process::exit(1);
    }

    Ok(())
}

fn progress_bar(mut rx: mpsc::Receiver<Message>, length: u64) -> Result<()> {
    let bar = ProgressBar::new(length);

    bar.set_style(
        ProgressStyle::with_template(
            "{prefix}[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}"
        )?.progress_chars("##-")
    );

    while let Some(message) = rx.blocking_recv() {
        match message {
            Message::Download(file_name, prefix) => {
                bar.inc(1);

                bar.set_message(file_name);

                if let Some(pre) = prefix {
                    bar.set_prefix(pre);
                }
            }
            Message::Stats(stats) => bar.set_message(stats.to_string()),
        }
    }

    bar.finish();

    Ok(())
}
