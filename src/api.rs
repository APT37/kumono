use crate::{ cli::ARGUMENTS, file::{ PostFile, PostFileRaw }, http::CLIENT, target::Target };
use anyhow::{ Result, format_err };
use regex::Regex;
use reqwest::StatusCode;
use serde::{ Deserialize, de::DeserializeOwned };
use std::{ fmt::Write, mem, sync::LazyLock };
use thiserror::Error;
use tokio::time::{ Duration, sleep };

const API_DELAY: Duration = Duration::from_millis(100);

static RE_OUT_OF_BOUNDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\{"error":"Offset [0-9]+ is bigger than total count [0-9]+\."\}"#).unwrap()
});

async fn try_fetch<D: DeserializeOwned>(url: &str) -> Result<D, ApiError> {
    sleep(API_DELAY).await;

    let res = CLIENT.get(url)
        .send().await
        .map_err(|err| ApiError::Connect(err.to_string()))?;

    let status = res.status();
    let text = res.text().await.expect("get text from response body");

    if status == StatusCode::BAD_REQUEST && RE_OUT_OF_BOUNDS.is_match(&text) {
        return Ok(serde_json::from_str("[]").unwrap());
    } else if status != StatusCode::OK {
        return Err(ApiError::Status(status));
    }

    serde_json::from_str(&text).map_err(|err| ApiError::Parser(err.to_string()))
}

pub trait Post {
    fn files(&mut self) -> Vec<PostFile>;
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("connection error")] Connect(String),
    #[error("non-success status code")] Status(StatusCode),
    #[error("post parsing failed")] Parser(String),
}

impl ApiError {
    pub async fn try_interpret(&self, retries: usize) -> Result<()> {
        async fn try_wait(retries: usize, duration: Duration, error: &str) -> Result<()> {
            if retries <= ARGUMENTS.max_retries {
                sleep(duration).await;
                Ok(())
            } else {
                Err(format_err!("{error}"))
            }
        }

        match self {
            ApiError::Connect(err) | ApiError::Parser(err) => {
                try_wait(retries, ARGUMENTS.retry_delay, err).await?;
            }
            ApiError::Status(status) =>
                match status.as_u16() {
                    403 | 429 | 502..=504 => {
                        try_wait(retries, ARGUMENTS.rate_limit_backoff, &status.to_string()).await?;
                    }
                    _ => try_wait(retries, ARGUMENTS.retry_delay, &status.to_string()).await?,
                }
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct SinglePost {
    post: SinglePostInner,
}

#[derive(Deserialize, Default)]
struct SinglePostInner {
    file: Option<PostFileRaw>,
    attachments: Vec<PostFileRaw>,
}

impl Post for SinglePost {
    fn files(&mut self) -> Vec<PostFile> {
        self.post.attachments.retain(|file| file.path.is_some());

        let attachments = mem::take(&mut self.post.attachments);

        let mut files = Vec::with_capacity(attachments.len() + 1);

        for raw in attachments {
            files.push(PostFile::new(raw.path.unwrap()));
        }

        if let Some(raw) = self.post.file.take() && let Some(path) = raw.path {
            files.push(PostFile::new(path));
        }

        files
    }
}

pub async fn try_fetch_page(
    target: &Target,
    user: &str,
    offset: usize
) -> Result<Vec<PagePost>, ApiError> {
    let (host, service, offset) = (
        target.as_service().host(),
        target.as_service().as_static_str(),
        offset.to_string(),
    );

    let mut url = String::with_capacity(
        8 + host.len() + 8 + service.len() + 6 + user.len() + 9 + offset.len()
    );

    write!(url, "https://{host}/api/v1/{service}/user/{user}/posts?o={offset}").unwrap();

    drop(offset);

    try_fetch(&url).await
}

#[derive(Deserialize)]
pub struct PagePost {
    file: Option<PostFileRaw>,
    attachments: Vec<PostFileRaw>,
}

impl Post for PagePost {
    fn files(&mut self) -> Vec<PostFile> {
        self.attachments.retain(|file| file.path.is_some());

        let attachments = mem::take(&mut self.attachments);

        let mut files = Vec::with_capacity(attachments.len() + 1);

        for raw in attachments {
            if let Some(path) = raw.path {
                files.push(PostFile::new(path));
            }
        }

        if let Some(raw) = self.file.take() && let Some(path) = raw.path {
            files.push(PostFile::new(path));
        }

        files
    }
}

#[derive(Deserialize)]
pub struct DiscordChannel {
    pub id: String, // "455285536341491716",
    // name: String, // "news"
}

pub async fn try_discord_server(server: &str) -> Result<Vec<DiscordChannel>, ApiError> {
    let mut url = String::with_capacity(48 + server.len());

    write!(url, "https://kemono.cr/api/v1/discord/channel/lookup/{server}").unwrap();

    try_fetch(&url).await
}

#[derive(Deserialize)]
pub struct DiscordPost {
    attachments: Vec<PostFileRaw>,
}

impl Post for DiscordPost {
    fn files(&mut self) -> Vec<PostFile> {
        self.attachments.retain(|file| file.path.is_some());

        let attachments = mem::take(&mut self.attachments);

        let mut files = Vec::with_capacity(attachments.len());

        for raw in attachments {
            files.push(PostFile::new(raw.path.unwrap()));
        }

        files
    }
}

pub async fn try_discord_page(channel: &str, offset: usize) -> Result<Vec<DiscordPost>, ApiError> {
    let offset = offset.to_string();

    let mut url = String::with_capacity(41 + channel.len() + 3 + offset.len());

    write!(url, "https://kemono.cr/api/v1/discord/channel/lookup/{channel}?o={offset}").unwrap();

    drop(offset);

    try_fetch(&url).await
}
