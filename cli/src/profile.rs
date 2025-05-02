use crate::{ cli::{ ARGS, Service }, client::CLIENT, n_fmt, stats::DownloadState };
use anyhow::{ Result, anyhow };
use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::Deserialize;
use sha256::async_digest::try_async_digest;
use size::Size;
use std::{ io::{ self, SeekFrom }, path::PathBuf };
use tokio::{ fs::File, io::{ AsyncSeekExt, AsyncWriteExt }, time::{ Duration, sleep } };

const API_DELAY: Duration = Duration::from_millis(100);

pub struct Profile<'a> {
    service: Service,
    creator_id: &'a str,
    posts: Vec<Post>,
    pub files: Vec<PostFile>,
}

impl<'a> Profile<'a> {
    pub async fn new(service: Service, creator_id: &'a str) -> Result<Self> {
        let mut profile = Self {
            service,
            creator_id,
            posts: vec![],
            files: vec![],
        };

        profile.init_posts().await?;
        profile.init_files();

        eprintln!(
            "found {} posts, containing {} files",
            n_fmt(profile.posts.len()),
            n_fmt(profile.files.len())
        );

        Ok(profile)
    }

    async fn init_posts(&mut self) -> Result<()> {
        eprintln!("fetching posts for {}/{}", self.service, self.creator_id);

        let mut offset = 0;

        loop {
            // debug!("fetching posts for {}/{} with offset {offset}", self.service, self.creator_id);

            let mut posts: Vec<Post>;

            let url = self.api_url_with_offset(offset);

            loop {
                sleep(API_DELAY).await;

                let response = CLIENT.get(&url).send().await?;

                match response.status() {
                    StatusCode::OK => {
                        posts = response.json().await?;
                        break;
                    }
                    StatusCode::TOO_MANY_REQUESTS => {
                        // warn!(
                        //     "hit rate-limit at offset {offset}, sleeping for {}",
                        //     pretty_duration::pretty_duration(&ARGS.api_backoff, None)
                        // );
                        sleep(ARGS.api_backoff).await;
                    }
                    status => {
                        return Err(anyhow!("{url} returned unexpected status: {status}"));
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
        format!(
            "https://{}.su/api/v1/{}/user/{}?o={offset}",
            self.service.site(),
            self.service,
            self.creator_id
        )
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
struct Post {
    // id: String,   // "1000537173"
    // this is the creator_id
    // user: String, // "paigetheuwulord"
    // service: Service, // "onlyfans"
    // title: String,     // "What an ass"
    file: PostFile,
    attachments: Vec<PostFile>,
}

impl Post {
    fn files(mut self) -> Vec<PostFile> {
        let mut files = vec![self.file];
        files.append(&mut self.attachments);
        files.retain(|pf| pf.path.is_some());
        files
    }
}

#[derive(Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PostFile {
    // original file name
    // name: Option<String>, // "1242x2208_882b040faaac0e38fba20f4caadb2e59.jpg",

    // remote file path, name is the file's sha256 hash
    path: Option<String>, // "/6e/6c/6e6cf84df44c1d091a2e36b6df77b098107c18831833de1e2e9c8207206f150b.jpg"
}

impl PostFile {
    fn to_url(&self, service: Service) -> String {
        format!("https://{}.su/data{}", service.site(), self.path.as_ref().unwrap())
    }

    fn to_pathbuf(&self, service: Service, creator_id: &str) -> PathBuf {
        PathBuf::from_iter([
            &service.to_string(),
            creator_id,
            self.path
                .as_ref()
                .unwrap()
                .split('/')
                .next_back()
                .expect("get local name from split remote path"),
        ])
    }

    pub fn to_name(&self, service: Service, creator_id: &str) -> String {
        self.to_pathbuf(service, creator_id)
            .file_name()
            .expect("get local file name from pathbuf")
            .to_string_lossy()
            .to_string()
    }

    async fn open(&self, service: Service, creator_id: &str) -> Result<File, io::Error> {
        File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(&self.to_pathbuf(service, creator_id)).await
    }

    pub async fn download(&self, service: Service, creator_id: &str) -> Result<DownloadState> {
        let s = |n| Size::from_bytes(n);

        let mut file = self.open(service, creator_id).await?;

        let initial_size = file.seek(SeekFrom::End(0)).await?;

        let mut local = initial_size;

        let remote = self.remote_size(service).await?;

        let name = self.to_name(service, creator_id);

        // let name_and_size = format!("{name} ({})", s(remote));

        if local == remote {
            return if
                ARGS.skip_initial_hash_verification ||
                name[..64] == try_async_digest(&self.to_pathbuf(service, creator_id)).await?
            {
                // debug!("skipping {name_and_size}");
                Ok(DownloadState::Skip)
            } else {
                Err(anyhow!("hash mismatch: {name} (before)"))
            };
            // } else if local == 0 {
            // debug!("downloading {name_and_size}");
            // } else {
            // debug!("resuming {name_and_size} [{} remaining]", s(remote - local));
        }

        loop {
            if let Err(err) = self.download_range(&mut file, service, local, remote - 1).await {
                let error = format!("{err}{}", if let Some(s) = err.source() {
                    format!("\n{s}")
                } else {
                    String::new()
                });
                return Ok(DownloadState::Failure(s(local - initial_size), Some(error)));
            }

            match file.seek(SeekFrom::End(0)).await {
                Ok(pos) => {
                    local = pos;
                }
                Err(err) => {
                    return Ok(
                        DownloadState::Failure(s(local - initial_size), Some(err.to_string()))
                    );
                }
            }

            if local == remote {
                break;
            }
        }

        let downloaded = s(local - initial_size);

        Ok(
            if name[..64] == sha256::try_digest(&self.to_pathbuf(service, creator_id))? {
                // debug!("skipping {name_and_size}");
                DownloadState::Success(downloaded)
            } else {
                DownloadState::Failure(downloaded, Some(format!("hash mismatch: {name} (after)")))
            }
        )
    }

    async fn download_range(
        &self,
        file: &mut File,
        service: Service,
        start: u64,
        end: u64
    ) -> Result<()> {
        let url = self.to_url(service);

        let mut first_error = true;

        loop {
            let response = CLIENT.get(&url)
                .header("Range", format!("bytes={start}-{end}"))
                .send().await?;

            let status = response.status();

            match status {
                StatusCode::PARTIAL_CONTENT => {
                    let mut stream = response.bytes_stream();

                    while let Some(Ok(bytes)) = stream.next().await {
                        file.write_all(&bytes).await?;
                    }

                    file.flush().await?;

                    break Ok(());
                }
                StatusCode::TOO_MANY_REQUESTS => sleep(ARGS.download_backoff).await,
                _ => {
                    if !first_error {
                        break Err(
                            anyhow!(
                                "[{status}] ({url}) failed to download range: server returned unexpected status codes repeatedly"
                            )
                        );
                    }

                    first_error = false;
                }
            }
        }
    }

    async fn remote_size(&self, service: Service) -> Result<u64> {
        fn size_error(url: &str, status: StatusCode, message: &str) -> Result<u64> {
            Err(anyhow!("[{status}] ({url}) failed to determine remote size: {message}"))
        }

        let mut first_error = true;

        loop {
            let response = CLIENT.head(self.to_url(service)).send().await?;

            let status = response.status();

            match status {
                StatusCode::OK => {
                    return match response.content_length() {
                        Some(length) => Ok(length),
                        None => {
                            return size_error(
                                self.path.as_ref().unwrap(),
                                status,
                                "Content-Length header is not present"
                            );
                        }
                    };
                }

                StatusCode::TOO_MANY_REQUESTS => sleep(ARGS.download_backoff).await,

                | StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::GATEWAY_TIMEOUT
                | StatusCode::BAD_GATEWAY => {
                    if first_error {
                        first_error = false;
                        sleep(ARGS.download_backoff).await;
                    } else {
                        size_error(
                            self.path.as_ref().unwrap(),
                            status,
                            "server returned errors repeatedly"
                        )?;
                    }
                }

                _ => {
                    return size_error(
                        self.path.as_ref().unwrap(),
                        status,
                        "received unexpected status code"
                    );
                }
            }
        }
    }
}
