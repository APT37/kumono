use crate::{ cli::ARGUMENTS, file::PostFile, http::CLIENT, target::Target };
use anyhow::{ Result, anyhow };
use regex::Regex;
use reqwest::StatusCode;
use serde::{ Deserialize, de::DeserializeOwned };
use std::sync::LazyLock;
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
    }

    if status != StatusCode::OK {
        return Err(ApiError::Status(status));
    }

    serde_json::from_str(&text).map_err(|err| ApiError::Parser(err.to_string()))
}

pub trait Post {
    fn files(&mut self) -> Vec<PostFile>;
}

#[derive(Debug, Clone, Error, PartialEq, Eq, PartialOrd, Ord)]
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
                Err(anyhow!("{error}"))
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SinglePost {
    post: SinglePostInner,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct SinglePostInner {
    file: Option<PostFile>,
    attachments: Vec<PostFile>,
}

impl Post for SinglePost {
    fn files(&mut self) -> Vec<PostFile> {
        self.post.attachments.retain(PostFile::has_path);

        let mut files = Vec::with_capacity(self.post.attachments.len() + 1);

        files.append(&mut self.post.attachments);

        if let Some(file) = self.post.file.take() && file.has_path() {
            files.push(file);
        }

        files
    }
}

pub async fn try_fetch_page(
    target: &Target,
    user: &str,
    offset: usize
) -> Result<Vec<PagePost>, ApiError> {
    try_fetch(
        &format!(
            "https://{site}/api/v1/{service}/user/{user}/posts?o={offset}",
            site = target.as_service().site(),
            service = target.as_service()
        )
    ).await
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PagePost {
    file: Option<PostFile>,
    attachments: Vec<PostFile>,
}

impl Post for PagePost {
    fn files(&mut self) -> Vec<PostFile> {
        self.attachments.retain(PostFile::has_path);

        let mut files = Vec::with_capacity(self.attachments.len() + 1);

        files.append(&mut self.attachments);

        if let Some(file) = self.file.take() && file.has_path() {
            files.push(file);
        }

        files
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscordChannel {
    pub id: String, // "455285536341491716",
    // name: String, // "news"
}

pub async fn try_discord_server(server: &str) -> Result<Vec<DiscordChannel>, ApiError> {
    try_fetch(&format!("https://kemono.cr/api/v1/discord/channel/lookup/{server}")).await
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscordPost {
    attachments: Vec<PostFile>,
}

impl Post for DiscordPost {
    fn files(&mut self) -> Vec<PostFile> {
        self.attachments
            .drain(..)
            .filter(PostFile::has_path)
            .collect()
    }
}

pub async fn try_discord_page(channel: &str, offset: usize) -> Result<Vec<DiscordPost>, ApiError> {
    try_fetch(&format!("https://kemono.cr/api/v1/discord/channel/{channel}?o={offset}")).await
}
