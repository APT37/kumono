use crate::{ file::PostFile, http::CLIENT, target::Target, cli::ARGUMENTS };
use anyhow::{ bail, Result };
use regex::Regex;
use reqwest::StatusCode;
use serde::{ de::DeserializeOwned, Deserialize };
use std::sync::LazyLock;
use thiserror::Error;
use tokio::time::{ Duration, sleep };

const API_DELAY: Duration = Duration::from_millis(100);

static RE_OUT_OF_BOUNDS: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r#"\{"error":"Offset [0-9]+ is bigger than total count [0-9]+\."\}"#).unwrap()
);

async fn fetch<T: DeserializeOwned>(url: &str) -> Result<T, ApiError> {
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
    pub async fn interpret(&self, retries: usize) -> Result<()> {
        async fn wait_or_bail(retries: usize, duration: Duration, error: &str) -> Result<()> {
            if retries < ARGUMENTS.max_retries {
                sleep(duration).await;
            } else {
                bail!("{error}");
            }

            Ok(())
        }

        match self {
            ApiError::Connect(err) | ApiError::Parser(err) =>
                wait_or_bail(retries, ARGUMENTS.retry_delay, err).await?,
            ApiError::Status(status) =>
                match status.as_u16() {
                    403 | 429 | 502..=504 =>
                        wait_or_bail(
                            retries,
                            ARGUMENTS.rate_limit_backoff,
                            &status.to_string()
                        ).await?,
                    _ => wait_or_bail(retries, ARGUMENTS.retry_delay, &status.to_string()).await?,
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
        let mut files = Vec::new();
        if let Some(file) = self.post.file.as_ref() {
            files.push(file.clone());
        }
        files.append(&mut self.post.attachments);
        files.retain(PostFile::has_path);
        files
    }
}

pub async fn page(target: &Target, user: &str, offset: usize) -> Result<Vec<PagePost>, ApiError> {
    fetch(
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
        let mut files = Vec::new();
        if let Some(file) = self.file.as_ref() {
            files.push(file.clone());
        }
        files.append(&mut self.attachments);
        files.retain(PostFile::has_path);
        files
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscordChannel {
    pub id: String, // "455285536341491716",
    // name: String, // "news"
}

pub async fn discord_server(server: &str) -> Result<Vec<DiscordChannel>, ApiError> {
    fetch(&format!("https://kemono.cr/api/v1/discord/channel/lookup/{server}")).await
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

pub async fn discord_page(channel: &str, offset: usize) -> Result<Vec<DiscordPost>, ApiError> {
    fetch(&format!("https://kemono.cr/api/v1/discord/channel/{channel}?o={offset}")).await
}
