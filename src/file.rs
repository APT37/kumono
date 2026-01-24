use crate::{ cli::ARGUMENTS, http::CLIENT, progress::DownloadAction, target::Target };
use anyhow::{ Context, Result, anyhow };
use futures_util::StreamExt;
use regex::Regex;
use reqwest::StatusCode;
use serde::Deserialize;
use std::{
    error::Error,
    fmt::{ self, Display, Formatter },
    io::SeekFrom,
    path::PathBuf,
    process::exit,
    sync::LazyLock,
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PostFileRaw {
    // Deserializing the name fieldbreaks our hashset's uniqueness guarantee;
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PostFile {
    pub name: String,
    path: String,
}

impl Display for PostFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PostFile {
    pub fn new(path: String) -> Self {
        Self {
            name: PathBuf::from(&path)
                .file_name()
                .expect("get file name from CDN path")
                .to_string_lossy()
                .to_string(),
            path,
        }
    }

    pub fn to_url(&self, target: &Target) -> String {
        format!("https://{site}/data{path}", site = target.as_service().site(), path = self.path)
    }

    pub fn to_temp_name(&self) -> String {
        format!("{}.temp", self.name)
    }

    pub fn to_extension(&self, target: &Target) -> Option<String> {
        self.to_pathbuf(target)
            .extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
    }

    pub fn to_pathbuf(&self, target: &Target) -> PathBuf {
        target.to_pathbuf(Some(&self.name))
    }

    pub fn to_temp_pathbuf(&self, target: &Target) -> PathBuf {
        target.to_pathbuf(Some(&self.to_temp_name()))
    }

    pub fn to_hash(&self) -> Option<String> {
        Some(HASH_RE.captures(&self.name)?.name("hash")?.as_str().to_string())
    }

    pub async fn try_open(&self, target: &Target) -> Result<File> {
        File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(&self.to_temp_pathbuf(target)).await
            .with_context(|| format!("Failed to open temporary file: {}", self.to_temp_name()))
    }

    /// Calculates the file's SHA256 hash
    pub async fn hash(&self, target: &Target) -> Result<String> {
        sha256
            ::try_async_digest(&self.to_temp_pathbuf(target)).await
            .with_context(|| format!("hash tempfile: {}", self.to_temp_name()))
    }

    pub async fn try_exists(&self, target: &Target) -> Result<bool> {
        fs::try_exists(self.to_pathbuf(target)).await.with_context(||
            format!("check if file exists: {}", self.to_temp_name())
        )
    }

    pub async fn try_move(&self, target: &Target) -> Result<()> {
        fs::rename(self.to_temp_pathbuf(target), self.to_pathbuf(target)).await.with_context(|| {
            format!(
                "rename tempfile to file: {temp_name} -> {name}",
                temp_name = self.to_temp_name(),
                name = self.name
            )
        })
    }

    pub async fn try_delete(&self, target: &Target) -> Result<()> {
        fs::remove_file(self.to_temp_pathbuf(target)).await.with_context(||
            format!("delete tempfile: {}", self.to_temp_name())
        )
    }

    pub async fn try_download(
        &self,
        target: &Target,
        mut msg_tx: Sender<DownloadAction>
    ) -> Result<DownloadAction> {
        msg_tx.send(DownloadAction::Start).await?;

        if self.try_exists(target).await? {
            return Ok(DownloadAction::Skip(self.to_hash(), self.to_extension(target)));
        }

        let (rsize, rpath) = self.try_fetch_remote_size_and_path(target, &mut msg_tx).await?;

        let mut temp_file = self.try_open(target).await?;

        let mut csize = temp_file.seek(SeekFrom::End(0)).await?;

        loop {
            if csize > rsize {
                self.try_delete(target).await?;

                return Ok(
                    DownloadAction::Fail(
                        format!(
                            "size mismatch (deleted): {name} [l: {csize} | r: {rsize}]",
                            name = self.name
                        ),
                        self.to_extension(target)
                    )
                );
            }

            if csize == rsize {
                break;
            }

            if
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
                return Ok(DownloadAction::Fail(error, self.to_extension(target)));
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
                    return Ok(DownloadAction::Fail(error, self.to_extension(target)));
                }
            }
        }

        Ok(
            if let Some(rhash) = self.to_hash() {
                let lhash = self.hash(target).await?;
                if rhash == lhash {
                    self.try_move(target).await?;
                    DownloadAction::Complete(Some(rhash), self.to_extension(target))
                } else {
                    self.try_delete(target).await?;
                    DownloadAction::Fail(
                        format!(
                            "hash mismatch (deleted): {name}\n| remote: {rhash}\n|  local: {lhash}",
                            name = self.name
                        ),
                        self.to_extension(target)
                    )
                }
            } else {
                msg_tx.send(DownloadAction::ReportLegacyHashSkip(self.name.clone())).await?;
                DownloadAction::Complete(None, self.to_extension(target))
            }
        )
    }

    pub async fn try_fetch_remote_size_and_path(
        &self,
        target: &Target,
        msg_tx: &mut Sender<DownloadAction>
    ) -> Result<(u64, String)> {
        fn produce_size_error(status: StatusCode, message: &str, url: &str) -> Result<u64> {
            Err(anyhow!("[{status}] remote size determination failed: {message} ({url})"))
        }

        let url = self.to_url(target);

        loop {
            let response = CLIENT.head(&url).send().await?;

            let status = response.status();

            if status == StatusCode::OK {
                let size = response
                    .content_length()
                    .map_or_else(
                        || produce_size_error(status, "Content-Length header is not present", &url),
                        Ok
                    )?;
                return Ok((size, response.url().to_string()));
            } else if status == StatusCode::NOT_FOUND {
                produce_size_error(status, "file not found", &url)?;
            } else if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
                try_wait(ARGUMENTS.rate_limit_backoff, msg_tx).await?;
            } else if status.is_server_error() {
                try_wait(ARGUMENTS.server_error_delay, msg_tx).await?;
            } else {
                produce_size_error(status, "unexpected status code", &url)?;
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
        fn produce_download_error(status: StatusCode, message: &str, url: &str) -> Result<()> {
            Err(anyhow!("[{status}] download failed: {message} ({url})"))
        }

        loop {
            let response = CLIENT.get(url).header("Range", format!("bytes={start}-")).send().await?;

            let status = response.status();

            if status == StatusCode::PARTIAL_CONTENT {
                let mut stream = response.bytes_stream();

                while let Some(Ok(bytes)) = stream.next().await {
                    msg_tx.send(DownloadAction::ReportSize(bytes.len() as u64)).await?;
                    if let Err(err) = file.write_all(&bytes).await {
                        msg_tx.send(
                            DownloadAction::Panic(format!("write error: {}\n{err}", self.name))
                        ).await?;
                        exit(1);
                    }
                }
                file.flush().await?;
                break Ok(());
            } else if status == StatusCode::NOT_FOUND {
                produce_download_error(status, "no file", url)?;
            } else if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
                try_wait(ARGUMENTS.rate_limit_backoff, msg_tx).await?;
            } else if status.is_server_error() {
                try_wait(ARGUMENTS.server_error_delay, msg_tx).await?;
            } else {
                produce_download_error(status, "unexpected status code", url)?;
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
