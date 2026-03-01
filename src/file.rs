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
    ops::Range,
    path::PathBuf,
    process::exit,
    sync::{ Arc, LazyLock },
    time::Duration,
};
use tokio::{
    fs::{ self, File },
    io::{ AsyncSeekExt, AsyncWriteExt },
    sync::mpsc::UnboundedSender,
    time::sleep,
};

const CHUNK_SIZE: u64 = 4 * 1024 * 1024; // 4 MiB

static HASH_RE: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"(?<hash>[0-9a-f]{64})(?:\..+)?$").unwrap()
);

#[derive(Deserialize, PartialEq, Eq, Hash, Debug)]
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
    base: String,
    path_range: Range<usize>,
    name_range: Range<usize>,
    temp_range: Range<usize>,
    pub ext_range: Option<Range<usize>>,
    pub hash_range: Option<Range<usize>>,
}

impl Display for PostFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

impl PostFile {
    pub fn new(path: String) -> Arc<Self> {
        let path_len = path.len();
        let path_range = Range {
            start: 0,
            end: path_len,
        };
        let path_buf = PathBuf::from(&path);

        let name = path_buf.file_name().expect("get file name from remote path").to_string_lossy();

        let name_range = Range {
            start: path.rfind(name.as_ref()).unwrap(),
            end: path_len,
        };

        let ext_range = path_buf.extension().map(|ext| Range {
            start: path.rfind(&ext.to_string_lossy().to_string()).unwrap(),
            end: path_len,
        });

        let hash_range = HASH_RE.captures(&path)
            .and_then(|c| c.name("hash"))
            .map(|c| c.range());

        let temp_range = Range {
            start: name_range.start,
            end: path_len + 5,
        };

        let base = path + ".temp";

        Arc::new(Self {
            base,
            path_range,
            name_range,
            temp_range,
            ext_range,
            hash_range,
        })
    }

    pub fn get_path(&self) -> &str {
        &self.base[self.path_range.start..self.path_range.end]
    }

    pub fn get_name(&self) -> &str {
        &self.base[self.name_range.start..self.name_range.end]
    }

    pub fn get_temp(&self) -> &str {
        &self.base[self.temp_range.start..self.temp_range.end]
    }

    pub fn get_ext(&self) -> Option<&str> {
        self.ext_range.as_ref().map(|e_r| &self.base[e_r.start..e_r.end])
    }

    pub fn get_hash(&self) -> Option<&str> {
        self.hash_range.as_ref().map(|h_r| &self.base[h_r.start..h_r.end])
    }

    pub fn to_url(&self, target: &Target) -> String {
        let host = target.as_service().host();
        let path = self.get_path();
        let mut url = String::with_capacity(8 + host.len() + 5 + path.len());
        let _ = write!(url, "https://{host}/data{path}");
        url
    }

    pub fn to_pathbuf(&self, target: &Target) -> PathBuf {
        let mut path = target.as_pathbuf().clone();
        path.push(self.get_name());
        path
    }

    pub fn to_temp_pathbuf(&self, target: &Target) -> PathBuf {
        let mut path = target.as_pathbuf().clone();
        path.push(self.get_temp());
        path
    }

    pub async fn try_open(&self, target: &Target) -> Result<File> {
        File::options()
            .append(true)
            .create(true)
            .truncate(false)
            .open(&self.to_temp_pathbuf(target)).await
            .with_context(|| {
                let mut buf = String::with_capacity(31 + self.temp_range.len());
                let _ = write!(buf, "Failed to open temporary file: {}", self.get_temp());
                buf
            })
    }

    pub async fn hash(&self, target: &Target) -> Result<String> {
        sha256::try_async_digest(&self.to_temp_pathbuf(target)).await.with_context(|| {
            let mut buf = String::with_capacity(15 + self.temp_range.len());
            let _ = write!(buf, "hash tempfile: {}", self.get_temp());
            buf
        })
    }

    pub async fn try_exists(&self, target: &Target) -> Result<bool> {
        fs::try_exists(self.to_pathbuf(target)).await.with_context(|| {
            let mut buf = String::with_capacity(22 + self.temp_range.len());
            let _ = write!(buf, "check if file exists: {}", self.get_temp());
            buf
        })
    }

    pub async fn try_move(&self, target: &Target) -> Result<()> {
        fs::rename(self.to_temp_pathbuf(target), self.to_pathbuf(target)).await.with_context(|| {
            let mut buf = String::with_capacity(29 + self.temp_range.len() + self.name_range.len());
            let _ = write!(
                buf,
                "rename tempfile to file: {} -> {}",
                self.get_temp(),
                self.get_name()
            );
            buf
        })
    }

    pub async fn try_delete(&self, target: &Target) -> Result<()> {
        fs::remove_file(self.to_temp_pathbuf(target)).await.with_context(|| {
            let mut buf = String::with_capacity(17 + self.temp_range.len());
            let _ = write!(buf, "delete tempfile: {}", self.get_temp());
            buf
        })
    }

    pub async fn try_download(
        file: Arc<PostFile>,
        target: &Target,
        mut msg_tx: UnboundedSender<DownloadAction>
    ) -> Result<DownloadAction> {
        msg_tx.send(DownloadAction::Start)?;

        if file.try_exists(target).await? {
            return Ok(DownloadAction::Skip(file.clone()));
        }

        let (rsize, rpath) = file.try_fetch_remote_size_and_path(target, &mut msg_tx).await?;

        let mut temp_file = file.try_open(target).await?;

        let mut csize = temp_file.seek(SeekFrom::End(0)).await?;

        loop {
            let mut range = String::with_capacity(32);

            if csize == rsize {
                break;
            } else if csize > rsize {
                file.try_delete(target).await?;

                return Ok(
                    DownloadAction::Fail(
                        {
                            let (csize, rsize) = (csize.to_string(), rsize.to_string());

                            let mut msg = String::with_capacity(
                                25 + file.name_range.len() + 5 + csize.len() + 6 + rsize.len() + 1
                            );
                            let _ = write!(
                                msg,
                                "size mismatch (deleted): {} [l: {csize} | r: {rsize}]",
                                file.get_name()
                            );

                            msg
                        },
                        file.clone()
                    )
                );
            } else if
                let Err(err) = file.try_download_range(
                    &rpath,
                    &mut temp_file,
                    &({
                        range.clear();
                        let _ = write!(
                            range,
                            "bytes={csize}-{}",
                            (rsize - 1).min(csize + CHUNK_SIZE - 1)
                        );
                        range
                    }),
                    &mut msg_tx
                ).await
            {
                let mut error = err.to_string();
                if let Some(src) = err.source() {
                    error.push('\n');
                    error.push_str(&src.to_string());
                }
                return Ok(DownloadAction::Fail(error, file.clone()));
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
                    return Ok(DownloadAction::Fail(error, file.clone()));
                }
            }
        }

        Ok(
            if let Some(rhash) = file.get_hash() {
                let lhash = file.hash(target).await?;
                if rhash == lhash {
                    file.try_move(target).await?;
                    DownloadAction::Complete(file.clone())
                } else {
                    file.try_delete(target).await?;
                    DownloadAction::Fail(
                        {
                            let mut msg = String::with_capacity(
                                25 + file.name_range.len() + 11 + rhash.len() + 10 + lhash.len()
                            );
                            let _ = write!(
                                msg,
                                "hash mismatch (deleted): {file}\n| remote: {rhash}\n|  local: {lhash}"
                            );

                            msg
                        },
                        file.clone()
                    )
                }
            } else {
                msg_tx.send(DownloadAction::ReportLegacyHashSkip(file.clone()))?;
                DownloadAction::Complete(file.clone())
            }
        )
    }

    pub async fn try_fetch_remote_size_and_path(
        &self,
        target: &Target,
        msg_tx: &mut UnboundedSender<DownloadAction>
    ) -> Result<(u64, String)> {
        fn size_error(status: StatusCode, message: &str, url: &str) -> Result<u64> {
            Err(format_err!("[{status}] remote size determination failed: {message} ({url})"))
        }

        let url = self.to_url(target);

        loop {
            let response = CLIENT.head(&url).send().await?;

            match response.status() {
                status if status == StatusCode::OK => {
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
                status if status.is_server_error() => {
                    try_wait(ARGUMENTS.server_error_delay, msg_tx).await?;
                }
                status => {
                    size_error(status, "unexpected status code", &url)?;
                }
            }
        }
    }

    async fn try_download_range(
        &self,
        url: &str,
        file: &mut File,
        range: &str,
        msg_tx: &mut UnboundedSender<DownloadAction>
    ) -> Result<()> {
        fn download_error(status: StatusCode, message: &str, url: &str) -> Result<()> {
            Err(format_err!("[{status}] download failed: {message} ({url})"))
        }

        loop {
            let response = CLIENT.get(url).header("Range", range).send().await?;

            match response.status() {
                StatusCode::PARTIAL_CONTENT => {
                    let mut stream = response.bytes_stream();

                    while let Some(Ok(bytes)) = stream.next().await {
                        msg_tx.send(DownloadAction::ReportSize(bytes.len() as u64))?;
                        if let Err(err) = file.write_all(&bytes).await {
                            msg_tx.send({
                                let error = err.to_string();
                                let mut buf = String::with_capacity(
                                    13 + self.name_range.len() + 1 + error.len()
                                );
                                let _ = write!(buf, "write error: {}\n{error}", self.get_name());
                                DownloadAction::Panic(buf)
                            })?;
                            exit(1);
                        }
                    }
                    file.flush().await?;
                    break Ok(());
                }
                StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS | StatusCode::NOT_FOUND => {
                    try_wait(ARGUMENTS.rate_limit_backoff, msg_tx).await?;
                }
                status if status.is_server_error() => {
                    try_wait(ARGUMENTS.server_error_delay, msg_tx).await?;
                }
                status => {
                    download_error(status, "unexpected status code", url)?;
                }
            }
        }
    }
}

async fn try_wait(duration: Duration, msg_tx: &mut UnboundedSender<DownloadAction>) -> Result<()> {
    msg_tx.send(DownloadAction::Wait)?;
    sleep(duration).await;
    msg_tx.send(DownloadAction::Continue)?;
    Ok(())
}
