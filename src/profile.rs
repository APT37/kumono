use crate::{ cli::ARGS, http::CLIENT, progress::{ DownloadState, n_fmt } };
use anyhow::{ Result, bail };
use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::Deserialize;
use size::Size;
use std::{ error::Error, fmt, io::{ self, SeekFrom }, path::PathBuf };
use tokio::{ fs::{ self, File }, io::{ AsyncSeekExt, AsyncWriteExt }, time::{ Duration, sleep } };

const API_DELAY: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct Profile {
    posts: Vec<Post>,
    pub files: Vec<PostFile>,
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let posts = match self.posts.len() {
            0 => "no posts",
            1 => "1 post",
            n => &(n_fmt(n) + " posts"),
        };

        let files = if self.posts.is_empty() {
            ""
        } else {
            match self.files.len() {
                0 => ", but no files",
                1 => " with 1 file",
                n => &format!(" with {} files", n_fmt(n)),
            }
        };

        write!(f, "{} user '{}' has {posts}{files}", ARGS.service, ARGS.user_id)
    }
}

impl Profile {
    pub async fn init() -> Result<Self> {
        let mut profile = Self::default();

        profile.init_posts().await?;
        profile.init_files();

        eprintln!("{profile}");

        profile.posts.clear();

        Ok(profile)
    }

    async fn init_posts(&mut self) -> Result<()> {
        let mut offset = 0;

        loop {
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
                        sleep(ARGS.rate_limit_backoff).await;
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
    pub fn to_url(&self) -> String {
        format!("https://{}.su/data{}", ARGS.service.site(), self.path.as_ref().unwrap())
    }

    pub fn to_name(&self) -> String {
        self.path
            .as_ref()
            .unwrap()
            .split('/')
            .next_back()
            .expect("get local name from split remote path")
            .to_string()
    }

    pub fn to_temp_name(&self) -> String {
        self.to_name() + ".temp"
    }

    pub fn to_extension(&self) -> Option<String> {
        self.to_pathbuf()
            .extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
    }

    pub fn to_pathbuf(&self) -> PathBuf {
        ARGS.to_pathbuf_with_file(self.to_name())
    }

    pub fn to_temp_pathbuf(&self) -> PathBuf {
        ARGS.to_pathbuf_with_file(self.to_temp_name())
    }

    pub fn to_hash(&self) -> String {
        self.to_name()[..64].to_string()
    }

    pub async fn open(&self) -> Result<File, io::Error> {
        File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(&self.to_temp_pathbuf()).await
    }

    pub async fn hash(&self) -> io::Result<String> {
        sha256::try_async_digest(&self.to_temp_pathbuf()).await
    }

    pub async fn exists(&self) -> io::Result<bool> {
        fs::try_exists(self.to_pathbuf()).await
    }

    pub async fn r#move(&self) -> io::Result<()> {
        fs::rename(self.to_temp_pathbuf(), self.to_pathbuf()).await
    }

    pub async fn delete(&self) -> io::Result<()> {
        fs::remove_file(self.to_temp_pathbuf()).await
    }

    pub async fn download(&self) -> Result<DownloadState> {
        let s = |n| Size::from_bytes(n);

        if self.exists().await? {
            return Ok(DownloadState::Skip);
        }

        let mut temp_file = self.open().await?;

        let isize = temp_file.seek(SeekFrom::End(0)).await?;

        let mut csize = isize;

        let rsize = self.remote_size().await?;

        loop {
            if csize == rsize {
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
                Ok(cursor) => {
                    csize = cursor;
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
            let dsize = s(csize - isize);

            if self.to_hash() == self.hash().await? {
                self.r#move().await?;
                DownloadState::Success(dsize)
            } else {
                self.delete().await?;
                DownloadState::Failure(
                    dsize,
                    format!("hash mismatch (deleted): {}", self.to_name())
                )
            }
        })
    }

    async fn download_range(&self, file: &mut File, start: u64) -> Result<()> {
        let url = self.to_url();

        loop {
            let response = CLIENT.get(&url)
                .header("Range", format!("bytes={start}-"))
                .send().await?;

            let status = response.status();

            if status == StatusCode::PARTIAL_CONTENT {
                let mut stream = response.bytes_stream();

                while let Some(Ok(bytes)) = stream.next().await {
                    file.write_all(&bytes).await?;
                }

                file.flush().await?;

                break Ok(());
            } else if status == StatusCode::NOT_FOUND {
                bail!("[{status}] download failed ({url})");
            } else if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
                sleep(ARGS.rate_limit_backoff).await;
            } else if status.is_server_error() {
                sleep(ARGS.server_error_delay).await;
            } else {
                bail!("[{status}] download failed: unexpected status code {url}");
            }
        }
    }

    pub async fn remote_size(&self) -> Result<u64> {
        fn size_error(status: StatusCode, message: &str, url: &str) -> Result<u64> {
            bail!("[{status}] failed to determine remote size: {message} ({url})")
        }

        let url = self.to_url();

        loop {
            let response = CLIENT.head(&url).send().await?;

            let status = response.status();

            if status == StatusCode::OK {
                return match response.content_length() {
                    Some(length) => Ok(length),
                    None => {
                        return size_error(status, "Content-Length header is not present", &url);
                    }
                };
            } else if status == StatusCode::NOT_FOUND {
                size_error(status, "file not found", &url)?;
            } else if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
                sleep(ARGS.rate_limit_backoff).await;
            } else if status.is_server_error() {
                sleep(ARGS.server_error_delay).await;
            } else {
                size_error(status, "unexpected status code", &url)?;
            }
        }
    }
}
