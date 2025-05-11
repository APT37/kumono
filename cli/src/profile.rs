use crate::{ cli::ARGS, http::CLIENT, progress::{ DownloadState, n_fmt } };
use anyhow::{ Result, bail };
use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::Deserialize;
use size::Size;
use std::{ error::Error, fmt, io::{ self, SeekFrom }, path::PathBuf };
use tokio::{ fs::{ self, File }, io::{ AsyncSeekExt, AsyncWriteExt }, time::{ Duration, sleep } };

const API_DELAY: Duration = Duration::from_millis(100);

pub struct Profile {
    posts: Vec<Post>,
    pub files: Vec<PostFile>,
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let posts = match self.posts.len() {
            0 => "no posts",
            1 => "1 post",
            _ => &(n_fmt(self.posts.len()) + " posts"),
        };

        let files = if self.posts.is_empty() {
            ""
        } else {
            match self.files.len() {
                0 => ", but no files",
                1 => " with 1 file",
                _ => &format!(" with {} files", n_fmt(self.files.len())),
            }
        };

        write!(f, "{} user '{}' has {posts}{files}", ARGS.service, ARGS.user_id)
    }
}

impl Profile {
    pub async fn init() -> Result<Self> {
        let mut profile = Self {
            posts: vec![],
            files: vec![],
        };

        profile.init_posts().await?;
        profile.init_files();

        eprintln!("{profile}");

        Ok(profile)
    }

    async fn init_posts(&mut self) -> Result<()> {
        let mut offset = 0;

        loop {
            // debug!("fetching posts for {}/{} with offset {offset}", self.service, self.user_id);

            let mut posts: Vec<Post>;

            let url = Self::api_url_with_offset(offset);

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

                    status => bail!("{url} returned unexpected status: {status}"),
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

    fn api_url_with_offset(offset: u32) -> String {
        format!(
            "https://{}.su/api/v1/{}/user/{}?o={offset}",
            ARGS.service.site(),
            ARGS.service,
            ARGS.user_id
        )
    }

    fn init_files(&mut self) {
        self.posts
            .clone() // TODO don't clone posts
            .into_iter()
            .for_each(|post| self.files.append(&mut post.files()));

        self.files.sort();
        self.files.dedup();
    }
}

#[derive(Deserialize, Clone)]
struct Post {
    // coomer/kemono database id
    // id: String, // "1000537173"

    // service user name/id
    // user: String, // "paigetheuwulord"

    // service: Service, // "onlyfans"

    // post title
    // title: String, // "What an ass"
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
    // original source file name
    // name: Option<String>, // "1242x2208_882b040faaac0e38fba20f4caadb2e59.jpg",

    // remote file path, name is the file's sha256 hash
    path: Option<String>, // "/6e/6c/6e6cf84df44c1d091a2e36b6df77b098107c18831833de1e2e9c8207206f150b.jpg"
}

impl PostFile {
    fn to_url(&self) -> String {
        format!("https://{}.su/data{}", ARGS.service.site(), self.path.as_ref().unwrap())
    }

    fn to_name(&self) -> String {
        self.path
            .as_ref()
            .unwrap()
            .split('/')
            .next_back()
            .expect("get local name from split remote path")
            .to_string()
    }

    fn to_temp_name(&self) -> String {
        self.to_name() + ".temp"
    }

    fn to_pathbuf(&self) -> PathBuf {
        ARGS.to_pathbuf_with_file(self.to_name())
    }

    fn to_temp_pathbuf(&self) -> PathBuf {
        ARGS.to_pathbuf_with_file(self.to_temp_name())
    }

    fn to_hash(&self) -> String {
        self.to_name()[..64].to_string()
    }

    async fn open(&self) -> Result<File, io::Error> {
        File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(&self.to_temp_pathbuf()).await
    }

    async fn hash(&self) -> io::Result<String> {
        sha256::try_async_digest(&self.to_temp_pathbuf()).await
    }

    async fn r#move(&self) -> io::Result<()> {
        fs::rename(self.to_temp_pathbuf(), self.to_pathbuf()).await
    }

    async fn delete(&self) -> io::Result<()> {
        fs::remove_file(self.to_temp_pathbuf()).await
    }

    pub async fn download(&self) -> Result<DownloadState> {
        let s = |n| Size::from_bytes(n);

        if fs::try_exists(self.to_pathbuf()).await? {
            return Ok(DownloadState::Skip);
        }

        let mut temp_file = self.open().await?;

        let isize = temp_file.seek(SeekFrom::End(0)).await?;

        let mut csize = isize;

        loop {
            if csize == self.remote_size().await? {
                break;
            }

            if let Err(err) = self.download_range(&mut temp_file, csize).await {
                let mut error = err.to_string();
                if let Some(source) = err.source() {
                    error.push('\n');
                    error.push_str(&source.to_string());
                }
                return Ok(DownloadState::Failure(s(csize - isize), error));
            }

            match temp_file.seek(SeekFrom::End(0)).await {
                Ok(size) => {
                    csize = size;
                }
                Err(err) => {
                    let mut error = err.to_string();
                    if let Some(source) = err.source() {
                        error.push('\n');
                        error.push_str(&source.to_string());
                    }
                    return Ok(DownloadState::Failure(s(csize - isize), error));
                }
            }
        }

        Ok({
            let downloaded = s(csize - isize);

            if self.to_hash() == self.hash().await? {
                self.r#move().await?;
                DownloadState::Success(downloaded)
            } else {
                self.delete().await?;
                DownloadState::Failure(
                    downloaded,
                    format!("hash mismatch (deleted): {}", self.to_name())
                )
            }
        })
    }

    async fn download_range(&self, file: &mut File, start: u64) -> Result<()> {
        fn range_error(status: StatusCode, message: &str) -> Result<u64> {
            bail!("[{status}] failed to download range: {message}")
        }

        let mut first_error = true;

        let url = self.to_url();

        loop {
            let response = CLIENT.get(&url)
                .header("Range", format!("bytes={start}-"))
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

                | StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::GATEWAY_TIMEOUT
                | StatusCode::BAD_GATEWAY => {
                    if first_error {
                        first_error = false;
                        sleep(ARGS.download_backoff).await;
                    } else {
                        range_error(status, "server returned errors repeatedly")?;
                    }
                }

                _ => {
                    if !first_error {
                        range_error(status, "server returned unexpected status codes repeatedly")?;
                    }

                    first_error = false;
                }
            }
        }
    }

    async fn remote_size(&self) -> Result<u64> {
        fn size_error(status: StatusCode, message: &str) -> Result<u64> {
            bail!("[{status}] failed to determine remote size: {message}")
        }

        let mut first_error = true;

        loop {
            let response = CLIENT.head(self.to_url()).send().await?;

            let status = response.status();

            match status {
                StatusCode::OK => {
                    return match response.content_length() {
                        Some(length) => Ok(length),
                        None => {
                            return size_error(status, "Content-Length header is not present");
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
                        size_error(status, "server returned errors repeatedly")?;
                    }
                }

                _ => {
                    size_error(status, "received unexpected status code")?;
                }
            }
        }
    }
}
