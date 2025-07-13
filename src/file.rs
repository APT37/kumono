use crate::{ cli::ARGS, http::CLIENT, progress::DownloadState, target::Target };
use anyhow::{ Context, Result, bail };
use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::Deserialize;
use std::{ error::Error, io::SeekFrom, path::PathBuf };
use tokio::{ fs::{ self, File }, io::{ AsyncSeekExt, AsyncWriteExt }, time::sleep };

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PostFile {
    pub name: Option<String>,
    pub path: Option<String>,
}

impl PostFile {
    pub fn to_url(&self, target: &Target) -> String {
        format!("https://{}.su/data{}", target.as_service().site(), self.path.as_ref().unwrap())
    }

    pub fn to_name(&self) -> String {
        PathBuf::from(self.path.as_ref().expect("get path from PostFile"))
            .file_name()
            .expect("get file name from CDN path")
            .to_string_lossy()
            .to_string()
    }

    pub fn to_temp_name(&self) -> String {
        self.to_name() + ".temp"
    }

    pub fn to_extension(&self, target: &Target) -> Option<String> {
        self.to_pathbuf(target)
            .extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
    }

    pub fn to_pathbuf(&self, target: &Target) -> PathBuf {
        target.to_pathbuf(Some(&self.to_name()))
    }

    pub fn to_temp_pathbuf(&self, target: &Target) -> PathBuf {
        target.to_pathbuf(Some(&self.to_temp_name()))
    }

    /// Converts the file's name to a SHA256 hash.
    ///
    /// This does no work for legacy files, as their names are not hashes.
    pub fn to_hash(&self) -> String {
        self.to_name()[..64].to_string()
    }

    pub async fn open(&self, target: &Target) -> Result<File> {
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

    pub async fn exists(&self, target: &Target) -> Result<bool> {
        fs::try_exists(self.to_pathbuf(target)).await.with_context(||
            format!("check if file exists: {}", self.to_temp_name())
        )
    }

    pub async fn r#move(&self, target: &Target) -> Result<()> {
        fs::rename(self.to_temp_pathbuf(target), self.to_pathbuf(target)).await.with_context(|| {
            format!("rename tempfile to file: {} -> {}", self.to_temp_name(), self.to_name())
        })
    }

    pub async fn delete(&self, target: &Target) -> Result<()> {
        fs::remove_file(self.to_temp_pathbuf(target)).await.with_context(||
            format!("delete tempfile: {}", self.to_temp_name())
        )
    }

    pub async fn download(&self, target: &Target) -> Result<DownloadState> {
        if self.exists(target).await? {
            return Ok(DownloadState::Skip(self.to_hash()));
        }

        let rsize = self.remote_size(target).await?;

        let mut temp_file = self.open(target).await?;

        let isize = temp_file.seek(SeekFrom::End(0)).await?;

        let mut csize = isize;

        loop {
            if rsize == csize {
                break;
            }

            if let Err(err) = self.download_range(&mut temp_file, csize, target).await {
                let mut error = err.to_string();
                if let Some(source) = err.source() {
                    error.push('\n');
                    error.push_str(&source.to_string());
                }
                return Ok(DownloadState::Failure(csize - isize, error));
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
                    return Ok(DownloadState::Failure(csize - isize, error));
                }
            }
        }

        Ok({
            let dsize = csize - isize;

            let hash = self.to_hash();

            if hash == self.hash(target).await? {
                self.r#move(target).await?;
                DownloadState::Success(dsize, hash)
            } else {
                self.delete(target).await?;
                DownloadState::Failure(
                    dsize,
                    format!("hash mismatch (deleted): {}", self.to_name())
                )
            }
        })
    }

    async fn download_range(&self, file: &mut File, start: u64, target: &Target) -> Result<()> {
        fn download_error(status: StatusCode, message: &str, url: &str) -> Result<()> {
            bail!("[{status}] download failed: {message} ({url})")
        }

        let url = self.to_url(target);

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
                download_error(status, "no file", &url)?;
            } else if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
                sleep(ARGS.rate_limit_backoff).await;
            } else if status.is_server_error() {
                sleep(ARGS.server_error_delay).await;
            } else {
                download_error(status, "unexpected status code", &url)?;
            }
        }
    }

    pub async fn remote_size(&self, target: &Target) -> Result<u64> {
        fn size_error(status: StatusCode, message: &str, url: &str) -> Result<u64> {
            bail!("[{status}] remote size determination failed: {message} ({url})")
        }

        let url = self.to_url(target);

        loop {
            let response = CLIENT.head(&url).send().await?;

            let status = response.status();

            if status == StatusCode::OK {
                return response
                    .content_length()
                    .map_or_else(
                        || size_error(status, "Content-Length header is not present", &url),
                        Ok
                    );
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
