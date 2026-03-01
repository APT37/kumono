use crate::{ cli::ARGUMENTS, file::{ PostFile, PostFileRaw }, http::CLIENT };
use anyhow::{ Result, format_err };
use regex::Regex;
use reqwest::StatusCode;
use serde::{ Deserialize, de::DeserializeOwned };
use std::{ mem, sync::{ Arc, LazyLock } };
use thiserror::Error;
use tokio::time::{ Duration, sleep };

const API_DELAY: Duration = Duration::from_millis(100);

static RE_OUT_OF_BOUNDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\{"error":"Offset [0-9]+ is bigger than total count [0-9]+\."\}"#).unwrap()
});

pub async fn try_fetch<D: DeserializeOwned>(url: &str) -> Result<D, PostError> {
    sleep(API_DELAY).await;

    let res = CLIENT.get(url)
        .send().await
        .map_err(|err| PostError::Connect(err.to_string()))?;

    let status = res.status();

    let Ok(text) = res.text().await else {
        eprintln!("skipping page due to malformed response (server issue)");
        return Err(PostError::MalformedPage);
    };

    if status == StatusCode::BAD_REQUEST && RE_OUT_OF_BOUNDS.is_match(&text) {
        Ok(serde_json::from_str("[]").unwrap())
    } else if status != StatusCode::OK {
        Err(PostError::Status(status))
    } else {
        serde_json::from_str(&text).map_err(|err| PostError::MalformedPost(err.to_string()))
    }
}

pub trait Post {
    fn files(&mut self) -> Vec<Arc<PostFile>>;
}

#[derive(Debug, Error)]
pub enum PostError {
    #[error("connection error")] Connect(String),
    #[error("non-success status code")] Status(StatusCode),
    #[error("malformed page data")] MalformedPage,
    #[error("malformed post data")] MalformedPost(String),
}

impl PostError {
    pub async fn try_interpret(&self, retries: usize) -> Result<()> {
        async fn try_wait(retries: usize, duration: Duration, error: &str) -> Result<()> {
            if retries < ARGUMENTS.max_tries - 1 {
                sleep(duration).await;
                Ok(())
            } else {
                Err(format_err!("{error}"))
            }
        }

        match self {
            PostError::Connect(err) | PostError::MalformedPost(err) => {
                try_wait(retries, ARGUMENTS.retry_delay, err).await?;
            }
            PostError::Status(status) =>
                match status.as_u16() {
                    403 | 429 | 502..=504 => {
                        try_wait(retries, ARGUMENTS.rate_limit_backoff, status.as_str()).await?;
                    }
                    _ => try_wait(retries, ARGUMENTS.retry_delay, status.as_str()).await?,
                }
            PostError::MalformedPage => unreachable!(),
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
    fn files(&mut self) -> Vec<Arc<PostFile>> {
        self.post.attachments.retain(|file| file.path.is_some());

        if self.post.attachments.is_empty() && self.post.file.is_none() {
            return Vec::new();
        }

        let attachments = mem::take(&mut self.post.attachments);

        let mut post_files = Vec::with_capacity(attachments.len() + 1);

        for raw in attachments {
            post_files.push(PostFile::new(raw.path.unwrap()));
        }

        if let Some(raw) = self.post.file.take() && let Some(path) = raw.path {
            post_files.push(PostFile::new(path));
        }

        post_files
    }
}

#[derive(Deserialize)]
pub struct PagePost {
    file: Option<PostFileRaw>,
    attachments: Vec<PostFileRaw>,
}

impl Post for PagePost {
    fn files(&mut self) -> Vec<Arc<PostFile>> {
        self.attachments.retain(|file| file.path.is_some());

        if self.attachments.is_empty() && self.file.is_none() {
            return Vec::new();
        }

        let attachments = mem::take(&mut self.attachments);

        let mut post_files = Vec::with_capacity(attachments.len() + 1);

        for raw in attachments {
            if let Some(path) = raw.path {
                post_files.push(PostFile::new(path));
            }
        }

        if let Some(raw) = self.file.take() && let Some(path) = raw.path {
            post_files.push(PostFile::new(path));
        }

        post_files
    }
}

#[derive(Deserialize)]
pub struct DiscordChannel {
    pub id: String, // "455285536341491716",
    // name: String, // "news"
}

#[derive(Deserialize)]
pub struct DiscordPost {
    attachments: Vec<PostFileRaw>,
}

impl Post for DiscordPost {
    fn files(&mut self) -> Vec<Arc<PostFile>> {
        self.attachments.retain(|file| file.path.is_some());

        if self.attachments.is_empty() {
            return Vec::new();
        }

        let attachments = mem::take(&mut self.attachments);

        let mut post_files = Vec::with_capacity(attachments.len());

        for raw in attachments {
            post_files.push(PostFile::new(raw.path.unwrap()));
        }

        post_files
    }
}
