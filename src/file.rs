use crate::{ cli::ARGUMENTS, http::CLIENT, progress::DownloadAction, target::Target };
use anyhow::{ Context, Result, format_err };
use futures_util::StreamExt;
use regex::Regex;
use reqwest::StatusCode;
use serde::Deserialize;
use std::{
    error::Error,
    fmt::{ self, Display, Formatter, Write },
    io::SeekFrom,
    path::PathBuf,
    process::exit,
    sync::{ Arc, LazyLock },
    time::Duration,
};
use tokio::{
    fs::{ self, File },
    io::{ AsyncSeekExt, AsyncWriteExt },
    sync::mpsc::Sender,
    time::sleep,
};

static HASH_RE: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"^(?<hash>[0-9a-f]{64})(?:\..+)?$").unwrap()
);

#[derive(Deserialize, PartialEq, Eq, Hash)]
pub struct PostFileRaw {
    // Deserializing the name field breaks our hashset's uniqueness guarantee;
    // the same file may be known under different names, leading to a race
    // condition where multiple concurrent tasks write to the same file,
    // causing corruption & size mismatches. hash/size mismatches lead to
    // file deletion, where a second race condition can occur.
    // This corruption also causes offsets to be incorrect,
    // leading to HTTP 416 (Range Not Satisfiable) responses.
    //
    // To prevent issues stemming from redundant file names, storing
    // files in a separate subdirectory for each post or attaching the hash
    // to the file name can be considered.
    // This would require the current storage behavior to be changed, though
    // the addition of a dedicated mode for this behavior would also be possible.
    //
    // pub name: Option<String>,
    pub path: Option<String>,
}

#[derive(PartialEq, Eq, Hash)]
pub struct PostFile {
    path: String,
    name: Arc<String>,
    temp_name: String,
    pub hash: Arc<Option<String>>,
    pub extension: Arc<Option<String>>,
}

impl Display for PostFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PostFile {
    pub fn new(path: String, target: &Target) -> Self {
        let name = PathBuf::from(&path)
            .file_name()
            .expect("get file name from CDN path")
            .to_string_lossy()
            .to_string();
        let name = Arc::new(name);

        let mut temp_name = String::with_capacity(name.len() + 5);
        let _ = write!(temp_name, "{name}.temp");

        let get_hash = |name| Some(HASH_RE.captures(name)?.name("hash")?.as_str().to_string());
        let hash = Arc::new(get_hash(&name));

        let get_extension = |target: &Target| {
            target
                .to_pathbuf(Some(&name))
                .extension()
                .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        };
        let extension = Arc::new(get_extension(target));

        Self {
            path,
            name,
            temp_name,
            hash,
            extension,
        }
    }

    pub fn to_url(&self, target: &Target) -> String {
        let host = target.as_service().host();

        let mut url = String::with_capacity(8 + host.len() + 5 + self.path.len());
        let _ = write!(url, "https://{host}/data{}", self.path);

        url
    }

    pub fn to_pathbuf(&self, target: &Target) -> PathBuf {
        target.to_pathbuf(Some(&self.name))
    }

    pub fn to_temp_pathbuf(&self, target: &Target) -> PathBuf {
        target.to_pathbuf(Some(&self.temp_name))
    }

    pub async fn try_open(&self, target: &Target) -> Result<File> {
        File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(&self.to_temp_pathbuf(target)).await
            .with_context(|| {
                let mut buf = String::with_capacity(31 + self.temp_name.len());
                let _ = write!(buf, "Failed to open temporary file: {}", self.temp_name);
                buf
            })
    }

    /// Calculates the file's SHA256 hash
    pub async fn hash(&self, target: &Target) -> Result<String> {
        sha256::try_async_digest(&self.to_temp_pathbuf(target)).await.with_context(|| {
            let mut buf = String::with_capacity(15 + self.temp_name.len());
            let _ = write!(buf, "hash tempfile: {}", self.temp_name);
            buf
        })
    }

    pub async fn try_exists(&self, target: &Target) -> Result<bool> {
        fs::try_exists(self.to_pathbuf(target)).await.with_context(|| {
            let mut buf = String::with_capacity(22 + self.temp_name.len());
            let _ = write!(buf, "check if file exists: {}", self.temp_name);
            buf
        })
    }

    pub async fn try_move(&self, target: &Target) -> Result<()> {
        fs::rename(self.to_temp_pathbuf(target), self.to_pathbuf(target)).await.with_context(|| {
            let mut buf = String::with_capacity(29 + self.temp_name.len() + self.name.len());
            let _ = write!(buf, "rename tempfile to file: {} -> {}", self.temp_name, self.name);
            buf
        })
    }

    pub async fn try_delete(&self, target: &Target) -> Result<()> {
        fs::remove_file(self.to_temp_pathbuf(target)).await.with_context(|| {
            let mut buf = String::with_capacity(17 + self.temp_name.len());
            let _ = write!(buf, "delete tempfile: {}", self.temp_name);
            buf
        })
    }

    pub async fn try_download(
        &self,
        target: &Target,
        mut msg_tx: Sender<DownloadAction>
    ) -> Result<DownloadAction> {
        msg_tx.send(DownloadAction::Start).await?;

        if self.try_exists(target).await? {
            return Ok(DownloadAction::Skip(self.hash.clone(), self.extension.clone()));
        }

        let (rsize, rpath) = self.try_fetch_remote_size_and_path(target, &mut msg_tx).await?;

        let mut temp_file = self.try_open(target).await?;

        let mut csize = temp_file.seek(SeekFrom::End(0)).await?;

        loop {
            if csize == rsize {
                break;
            } else if csize > rsize {
                self.try_delete(target).await?;

                return Ok(
                    DownloadAction::Fail(
                        {
                            let (csize, rsize) = (csize.to_string(), rsize.to_string());

                            let mut msg = String::with_capacity(
                                25 + self.name.len() + 5 + csize.len() + 6 + rsize.len() + 1
                            );
                            let _ = write!(
                                msg,
                                "size mismatch (deleted): {} [l: {csize} | r: {rsize}]",
                                self.name
                            );

                            msg
                        },
                        self.extension.clone()
                    )
                );
            } else if
                let Err(err) = self.try_download_range(
                    &rpath,
                    &mut temp_file,
                    csize,
                    &mut msg_tx
                ).await
            {
                let mut error = err.to_string();
                if let Some(src) = err.source() {
                    error.push('\n');
                    error.push_str(&src.to_string());
                }
                return Ok(DownloadAction::Fail(error, self.extension.clone()));
            }

            match temp_file.seek(SeekFrom::End(0)).await {
                Ok(cursor) => {
                    csize = cursor;
                }
                Err(err) => {
                    let mut error = err.to_string();
                    if let Some(src) = err.source() {
                        error.push('\n');
                        error.push_str(&src.to_string());
                    }
                    return Ok(DownloadAction::Fail(error, self.extension.clone()));
                }
            }
        }

        Ok(
            if let Some(rhash) = self.hash.as_ref().as_deref() {
                let lhash = self.hash(target).await?;
                if rhash == lhash {
                    self.try_move(target).await?;
                    DownloadAction::Complete(self.hash.clone(), self.extension.clone())
                } else {
                    self.try_delete(target).await?;
                    DownloadAction::Fail(
                        {
                            let mut msg = String::with_capacity(
                                25 + self.name.len() + 11 + rhash.len() + 10 + lhash.len()
                            );
                            let _ = write!(
                                msg,
                                "hash mismatch (deleted): {name}\n| remote: {rhash}\n|  local: {lhash}",
                                name = self.name
                            );

                            msg
                        },
                        self.extension.clone()
                    )
                }
            } else {
                msg_tx.send(DownloadAction::ReportLegacyHashSkip(self.name.clone())).await?;
                DownloadAction::Complete(self.hash.clone(), self.extension.clone())
            }
        )
    }

    pub async fn try_fetch_remote_size_and_path(
        &self,
        target: &Target,
        msg_tx: &mut Sender<DownloadAction>
    ) -> Result<(u64, String)> {
        fn size_error(status: StatusCode, message: &str, url: &str) -> Result<u64> {
            Err(format_err!("[{status}] remote size determination failed: {message} ({url})"))
        }

        let url = self.to_url(target);

        loop {
            let response = CLIENT.head(&url).send().await?;
            let status = response.status();

            match status {
                StatusCode::OK => {
                    let size = response
                        .content_length()
                        .map_or_else(
                            || size_error(status, "Content-Length header is not present", &url),
                            Ok
                        )?;
                    return Ok((size, response.url().to_string()));
                }
                StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS | StatusCode::NOT_FOUND => {
                    try_wait(ARGUMENTS.rate_limit_backoff, msg_tx).await?;
                }
                _ if status.is_server_error() => {
                    try_wait(ARGUMENTS.server_error_delay, msg_tx).await?;
                }
                _ => {
                    size_error(status, "unexpected status code", &url)?;
                }
            }
        }
    }

    async fn try_download_range(
        &self,
        url: &str,
        file: &mut File,
        start: u64,
        msg_tx: &mut Sender<DownloadAction>
    ) -> Result<()> {
        fn download_error(status: StatusCode, message: &str, url: &str) -> Result<()> {
            Err(format_err!("[{status}] download failed: {message} ({url})"))
        }

        loop {
            let response = CLIENT.get(url)
                .header("Range", {
                    let mut range = String::with_capacity(32);
                    let _ = write!(range, "bytes={start}-");
                    range
                })
                .send().await?;
            let status = response.status();

            match status {
                StatusCode::PARTIAL_CONTENT => {
                    let mut stream = response.bytes_stream();

                    while let Some(Ok(bytes)) = stream.next().await {
                        msg_tx.send(DownloadAction::ReportSize(bytes.len() as u64)).await?;
                        if let Err(err) = file.write_all(&bytes).await {
                            msg_tx.send({
                                let error = err.to_string();
                                let mut buf = String::with_capacity(
                                    13 + self.name.len() + 1 + error.len()
                                );
                                let _ = write!(buf, "write error: {}\n{error}", self.name);
                                DownloadAction::Panic(buf)
                            }).await?;
                            exit(1);
                        }
                    }
                    file.flush().await?;
                    break Ok(());
                }
                StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS | StatusCode::NOT_FOUND => {
                    try_wait(ARGUMENTS.rate_limit_backoff, msg_tx).await?;
                }
                _ if status.is_server_error() => {
                    try_wait(ARGUMENTS.server_error_delay, msg_tx).await?;
                }
                _ => {
                    download_error(status, "unexpected status code", url)?;
                }
            }
        }
    }
}

async fn try_wait(duration: Duration, msg_tx: &mut Sender<DownloadAction>) -> Result<()> {
    msg_tx.send(DownloadAction::Wait).await?;
    sleep(duration).await;
    msg_tx.send(DownloadAction::Continue).await?;
    Ok(())
}
