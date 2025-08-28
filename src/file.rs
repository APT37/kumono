use crate::{ cli::ARGS, http::CLIENT, progress::DownloadAction, target::Target };
use anyhow::{ Context, Result, bail };
use futures_util::StreamExt;
use regex::Regex;
use reqwest::StatusCode;
use serde::Deserialize;
use std::{ error::Error, io::SeekFrom, path::PathBuf, sync::LazyLock, time::Duration };
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
pub struct PostFile {
    // deserializing the name breaks our hashset's uniqueness guarantee; the same file
    // may be known under different names, leading to a race condition where multiple
    // concurrent tasks write to the same file.
    // effects: corruption, size mismatch => deletion (2nd race condition), HTTP 426
    //
    // pub name: Option<String>,
    pub path: Option<String>,
    #[serde(skip)]
    pub service_override: Option<String>,
    #[serde(skip)]
    pub user_override: Option<String>,
}

impl PostFile {
    pub fn has_path(&self) -> bool {
        self.path.is_some()
    }

    pub fn to_url_with_service(&self, target: &Target, service_override: Option<&str>) -> String {
        let site = if let Some(service_str) = service_override {
            if let Ok(service) = service_str.parse::<crate::target::Service>() {
                service.site()
            } else {
                target.as_service().site()
            }
        } else {
            target.as_service().site()
        };

        format!(
            "https://{site}/data{path}",
            site = site,
            path = self.path.as_ref().unwrap()
        )
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
        // For favorites posts with service and user overrides, use the actual service/user path
        // instead of the generic "favorites" path to avoid duplicate downloads
        if let (Some(service_str), Some(user_str)) = (&self.service_override, &self.user_override) {
            if let Ok(service) = service_str.parse::<crate::target::Service>() {
                return PathBuf::from_iter([
                    &crate::cli::ARGS.output_path,
                    &service.to_string(),
                    user_str,
                    &self.to_name(),
                ]);
            }
        }
        
        target.to_pathbuf(Some(&self.to_name()))
    }

    pub fn to_temp_pathbuf(&self, target: &Target) -> PathBuf {
        // For favorites posts with service and user overrides, use the actual service/user path
        // instead of the generic "favorites" path to avoid duplicate downloads
        if let (Some(service_str), Some(user_str)) = (&self.service_override, &self.user_override) {
            if let Ok(service) = service_str.parse::<crate::target::Service>() {
                return PathBuf::from_iter([
                    &crate::cli::ARGS.output_path,
                    &service.to_string(),
                    user_str,
                    &self.to_temp_name(),
                ]);
            }
        }
        
        target.to_pathbuf(Some(&self.to_temp_name()))
    }

    pub fn to_hash(&self) -> Option<String> {
        let name = self.to_name();
        HASH_RE.captures(&name)?.name("hash").map(|m| m.as_str().to_string())
    }

    pub fn to_archive_pathbuf(&self, target: &Target) -> PathBuf {
        // For favorites posts with service and user overrides, use the actual service/user archive
        // instead of the generic "favorites" archive to match with direct creator downloads
        if let (Some(service_str), Some(user_str)) = (&self.service_override, &self.user_override) {
            if let Ok(service) = service_str.parse::<crate::target::Service>() {
                return PathBuf::from_iter([
                    &crate::cli::ARGS.output_path,
                    "db",
                    &format!("{}+{}.txt", service.to_string(), user_str),
                ]);
            }
        }
        
        target.to_archive_pathbuf()
    }

    pub fn load_archive(&self, target: &Target) -> Result<Vec<String>> {
        let archive_path = self.to_archive_pathbuf(target);
        
        if !std::path::Path::new(&archive_path).exists() {
            return Ok(Vec::new());
        }
        
        let contents = std::fs::read_to_string(&archive_path)
            .with_context(|| format!("Failed to read archive file: {}", archive_path.display()))?;
        
        Ok(contents.lines().map(str::to_string).collect())
    }

    pub fn is_in_archive(&self, target: &Target) -> Result<bool> {
        let archive = self.load_archive(target)?;
        if let Some(hash) = self.to_hash() {
            Ok(archive.contains(&hash))
        } else {
            Ok(false)
        }
    }

    pub async fn write_to_archive(&self, target: &Target, hash: &str) -> Result<()> {
        if !crate::cli::ARGS.download_archive {
            return Ok(());
        }
        
        use tokio::io::AsyncWriteExt;
        let archive_path = self.to_archive_pathbuf(target);
        
        // Create the db directory if it doesn't exist
        if let Some(parent) = archive_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&archive_path).await
            .with_context(|| format!("Failed to open archive file: {}", archive_path.display()))?;
        
        file.write_all(hash.as_bytes()).await
            .with_context(|| format!("Failed to write to archive file: {}", archive_path.display()))?;
        file.write_all(b"\n").await
            .with_context(|| format!("Failed to write newline to archive file: {}", archive_path.display()))?;
        
        Ok(())
    }

    pub async fn open(&self, target: &Target) -> Result<File> {
        let temp_path = self.to_temp_pathbuf(target);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = temp_path.parent() {
            fs::create_dir_all(parent).await
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        
        File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(&temp_path).await
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

    pub async fn download(
        &self,
        target: &Target,
        msg_tx: Sender<DownloadAction>
    ) -> Result<DownloadAction> {
        let service_override = self.service_override.as_deref();
        self.download_with_service(target, service_override, msg_tx).await
    }

    pub async fn download_with_service(
        &self,
        target: &Target,
        service_override: Option<&str>,
        mut msg_tx: Sender<DownloadAction>
    ) -> Result<DownloadAction> {
        msg_tx.send(DownloadAction::Start).await?;

        if self.exists(target).await? {
            return Ok(DownloadAction::Skip(self.to_hash()));
        }

        let rsize = self.remote_size_with_service(target, service_override, &mut msg_tx).await?;

        let mut temp_file = self.open(target).await?;

        let isize = temp_file.seek(SeekFrom::End(0)).await?;

        let mut csize = isize;

        loop {
            if csize > rsize {
                self.delete(target).await?;

                return Ok(
                    DownloadAction::Fail(
                        format!(
                            "size mismatch (deleted): {} [l: {csize} | r: {rsize}]",
                            self.to_name()
                        )
                    )
                );
            }

            if csize == rsize {
                break;
            }

            if let Err(err) = self.download_range_with_service(&mut temp_file, target, service_override, csize, &mut msg_tx).await {
                let mut error = err.to_string();
                if let Some(source) = err.source() {
                    error.push('\n');
                    error.push_str(&source.to_string());
                }
                return Ok(DownloadAction::Fail(error));
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
                    return Ok(DownloadAction::Fail(error));
                }
            }
        }

        Ok(
            if let Some(rhash) = self.to_hash() {
                let lhash = self.hash(target).await?;
                if rhash == lhash {
                    self.r#move(target).await?;
                    
                    // For favorites, write to the per-file archive instead of letting progress bar handle it
                    let hash_for_progress = match target {
                        crate::target::Target::Favorites { .. } => {
                            // Write to the service-specific archive file for this file
                            if let Err(err) = self.write_to_archive(target, &rhash).await {
                                return Err(err);
                            }
                            None // Don't let progress bar write to archive
                        }
                        _ => Some(rhash) // Let progress bar handle archive writing
                    };
                    
                    DownloadAction::Complete(hash_for_progress)
                } else {
                    self.delete(target).await?;
                    DownloadAction::Fail(
                        format!(
                            "hash mismatch (deleted): {}\n| remote: {rhash}\n| local:  {lhash}",
                            self.to_name()
                        )
                    )
                }
            } else {
                msg_tx.send(DownloadAction::ReportLegacyHashSkip(self.to_name())).await?;
                DownloadAction::Complete(None)
            }
        )
    }

    async fn download_range_with_service(
        &self,
        file: &mut File,
        target: &Target,
        service_override: Option<&str>,
        start: u64,
        msg_tx: &mut Sender<DownloadAction>
    ) -> Result<()> {
        fn download_error(status: StatusCode, message: &str, url: &str) -> Result<()> {
            bail!("[{status}] download failed: {message} ({url})")
        }

        let url = self.to_url_with_service(target, service_override);

        loop {
            let response = CLIENT.get(&url)
                .header("Range", format!("bytes={start}-"))
                .send().await?;

            let status = response.status();

            if status == StatusCode::PARTIAL_CONTENT {
                let mut stream = response.bytes_stream();

                while let Some(Ok(bytes)) = stream.next().await {
                    file.write_all(&bytes).await?;
                    msg_tx.send(DownloadAction::ReportSize(bytes.len() as u64)).await?;
                }
                file.flush().await?;

                break Ok(());
            } else if status == StatusCode::NOT_FOUND {
                download_error(status, "no file", &url)?;
            } else if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
                wait(ARGS.rate_limit_backoff, msg_tx).await?;
            } else if status.is_server_error() {
                wait(ARGS.server_error_delay, msg_tx).await?;
            } else {
                download_error(status, "unexpected status code", &url)?;
            }
        }
    }

    pub async fn remote_size_with_service(
        &self,
        target: &Target,
        service_override: Option<&str>,
        msg_tx: &mut Sender<DownloadAction>
    ) -> Result<u64> {
        fn size_error(status: StatusCode, message: &str, url: &str) -> Result<u64> {
            bail!("[{status}] remote size determination failed: {message} ({url})")
        }

        let url = self.to_url_with_service(target, service_override);

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
                wait(ARGS.rate_limit_backoff, msg_tx).await?;
            } else if status.is_server_error() {
                wait(ARGS.server_error_delay, msg_tx).await?;
            } else {
                size_error(status, "unexpected status code", &url)?;
            }
        }
    }
}

async fn wait(duration: Duration, msg_tx: &mut Sender<DownloadAction>) -> Result<()> {
    msg_tx.send(DownloadAction::Wait).await?;
    sleep(duration).await;
    msg_tx.send(DownloadAction::Continue).await?;
    Ok(())
}
