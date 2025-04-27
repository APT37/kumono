use crate::{ client::CLIENT, config::CONFIG, n_fmt, stats::DownloadState };
use anyhow::{ Result, anyhow };
use futures_util::StreamExt;
use log::{ debug, error, info, warn };
use reqwest::StatusCode;
use serde::Deserialize;
use size::Size;
use std::{ io::SeekFrom, path::PathBuf, process };
use tokio::{ fs::File, io::{ AsyncSeekExt, AsyncWriteExt }, time::sleep };

pub struct Profile<'a> {
    service: &'a str,
    creator: &'a str,
    pub posts: Vec<Post>,
    pub files: Vec<TargetFile>,
}

impl<'a> Profile<'a> {
    pub async fn new(service: &'a str, creator: &'a str) -> Result<Self> {
        let mut profile = Self {
            service,
            creator,
            posts: vec![],
            files: vec![],
        };

        profile.init_posts().await?;
        profile.init_files();

        info!(
            "found {} posts, containing {} files",
            n_fmt(profile.posts.len()),
            n_fmt(profile.files.len())
        );

        Ok(profile)
    }

    pub async fn init_posts(&mut self) -> Result<()> {
        info!("fetching posts for {}/{}", self.service, self.creator);

        let mut offset = 0;

        loop {
            debug!("fetching posts for {}/{} with offset {offset}", self.service, self.creator);

            let mut posts: Vec<Post>;

            let url = self.api_url_with_offset(offset);

            loop {
                sleep(CONFIG.api_delay_ms).await;

                let response = CLIENT.get(&url).send().await?;

                match response.status() {
                    StatusCode::OK => {
                        posts = response.json().await?;
                        break;
                    }
                    StatusCode::TOO_MANY_REQUESTS => {
                        warn!(
                            "hit rate-limit at offset {offset}, sleeping for {}",
                            pretty_duration::pretty_duration(&CONFIG.api_backoff, None)
                        );
                        sleep(CONFIG.api_backoff).await;
                    }
                    status => {
                        error!("{url} returned unexpected status: {status}");
                        process::exit(1);
                    }
                }
            }

            if posts.is_empty() {
                break;
            }

            self.posts.append(&mut posts);

            offset += 50;
        }

        Ok(())
    }

    fn api_url_with_offset(&self, offset: u32) -> String {
        format!("https://coomer.su/api/v1/{}/user/{}?o={offset}", self.service, self.creator)
    }

    fn init_files(&mut self) {
        self.posts
            .clone()
            .into_iter()
            .for_each(|post| self.files.append(&mut post.files()));

        self.files.sort();
        self.files.dedup();
    }
}

#[derive(Deserialize, Clone)]
pub struct Post {
    // id: String,   // "1000537173"
    pub user: String, // "paigetheuwulord"
    // service: String,  // "onlyfans"
    // title: String,     // "What an ass"
    file: PostFile,
    attachments: Vec<PostFile>,
}

impl Post {
    pub fn files(mut self) -> Vec<TargetFile> {
        let mut files = vec![self.file];

        files.append(&mut self.attachments);

        files
            .into_iter()
            .filter_map(|f| {
                if let (Some(name), Some(path)) = (f.name, f.path) {
                    Some(TargetFile::new(&self.user, &name, &path))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Deserialize, Clone)]
struct PostFile {
    name: Option<String>, // "1242x2208_882b040faaac0e38fba20f4caadb2e59.jpg",
    path: Option<String>, // "/6e/6c/6e6cf84df44c1d091a2e36b6df77b098107c18831833de1e2e9c8207206f150b.jpg"
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetFile {
    name: String,
    url: String, // "https://coomer.su/data/6e/6c/6e6cf84df44c1d091a2e36b6df77b098107c18831833de1e2e9c8207206f150b.jpg"
    fs_path: PathBuf,
}

impl TargetFile {
    fn new(creator: &str, filename: &str, path: &str) -> Self {
        Self {
            name: filename.to_string(),

            url: format!("https://coomer.su/data{path}"),

            fs_path: PathBuf::from_iter([creator, filename]),
        }
    }

    pub async fn download(&self) -> Result<DownloadState> {
        let s = |n| Size::from_bytes(n);

        let mut file = File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(&self.fs_path).await?;

        let initial_size = file.seek(SeekFrom::End(0)).await?;

        let mut local = initial_size;

        let remote = self.remote_size().await?;

        let name_size = format!("{} ({})", self.name, s(remote));

        if local == remote {
            debug!("skipping {name_size}");
            return Ok(DownloadState::Skip);
        }

        if local == 0 {
            info!("downloading {name_size}");
        } else {
            info!("resuming {name_size} [{} remaining]", s(remote - local));
        }

        loop {
            if let Err(err) = self.download_range(&mut file, local, remote - 1).await {
                error!("{err}");
                return Ok(DownloadState::Fail(s(local - initial_size)));
            }

            match file.seek(SeekFrom::End(0)).await {
                Ok(pos) => {
                    local = pos;
                }
                Err(err) => {
                    error!("{err}");
                    return Ok(DownloadState::Fail(s(local - initial_size)));
                }
            }

            if local == remote {
                file.flush().await?;
                break;
            }
        }

        Ok(DownloadState::Success(Size::from_bytes(local - initial_size)))
    }

    async fn remote_size(&self) -> Result<u64> {
        let mut first_error = true;

        loop {
            let response = CLIENT.head(&self.url).send().await?;

            match response.status() {
                StatusCode::OK => {
                    return match response.content_length() {
                        Some(length) => Ok(length),
                        None =>
                            Err(
                                anyhow!(
                                    "failed to determine remote size: Content-Length header is not present"
                                )
                            ),
                    };
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    warn!(
                        "hit rate-limit at {}, sleeping for {}",
                        self.url,
                        pretty_duration::pretty_duration(&CONFIG.download_backoff, None)
                    );
                    sleep(CONFIG.download_backoff).await;
                }
                StatusCode::GATEWAY_TIMEOUT => {
                    if first_error {
                        first_error = false;
                        warn!(
                            "gateway timed out at {}, sleeping for {}",
                            self.url,
                            pretty_duration::pretty_duration(&CONFIG.download_backoff, None)
                        );
                        sleep(CONFIG.download_backoff).await;
                    } else {
                        error!(
                            "failed to determine remote size: gateway timed out repeatedly ({})",
                            self.url
                        );
                        process::exit(1);
                    }
                }
                status => {
                    error!(
                        "failed to determine remote size: {} returned unexpected status: {status}",
                        self.url
                    );
                    process::exit(1);
                }
            }
        }
    }

    async fn download_range(&self, file: &mut File, start: u64, end: u64) -> Result<()> {
        let mut stream = CLIENT.get(&self.url)
            .header("Range", format!("bytes={start}-{end}"))
            .send().await?
            .bytes_stream();

        while let Some(Ok(bytes)) = stream.next().await {
            file.write_all(&bytes).await?;
        }

        Ok(())
    }
}
