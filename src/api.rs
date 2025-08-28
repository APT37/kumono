use crate::{ file::PostFile, http::CLIENT, target::Target, cli::ARGS };
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
            if retries < ARGS.max_retries {
                sleep(duration).await;
            } else {
                bail!("{error}");
            }

            Ok(())
        }

        match self {
            ApiError::Connect(err) => wait_or_bail(retries, ARGS.retry_delay, err).await?,
            ApiError::Status(status) =>
                match status.as_u16() {
                    403 | 429 | 502..=504 =>
                        wait_or_bail(retries, ARGS.rate_limit_backoff, &status.to_string()).await?,
                    _ => wait_or_bail(retries, ARGS.retry_delay, &status.to_string()).await?,
                }
            ApiError::Parser(err) => wait_or_bail(retries, ARGS.retry_delay, err).await?,
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SinglePost {
    post: SinglePostInner,
    // "attachments": [
    //   {
    //     "server": "https://n4.coomer.su",
    //     "name": "0hpxn0h31vg5siq0xtqsl_source.mp4",
    //     "extension": ".mp4",
    //     "name_extension": ".mp4",
    //     "stem": "7d1c604743b8bf30c7c5a260ac487ae1dea4ef133cc767ed0c5989c981d56823",
    //     "path": "/7d/1c/7d1c604743b8bf30c7c5a260ac487ae1dea4ef133cc767ed0c5989c981d56823.mp4"
    //   }
    // ],
    // "previews": [],
    // "videos": [
    //   {
    //     "index": 0,
    //     "path": "/7d/1c/7d1c604743b8bf30c7c5a260ac487ae1dea4ef133cc767ed0c5989c981d56823.mp4",
    //     "name": "0hpxn0h31vg5siq0xtqsl_source.mp4",
    //     "extension": ".mp4",
    //     "name_extension": ".mp4",
    //     "server": "https://n4.coomer.su"
    //   }
    // ],
    //     "props": {
    //       "flagged": "reason-other",
    //       "revisions": [
    //         [
    //           0,
    //           {
    //             "id": "1080444052",
    //             "user": "belledelphine",
    //             "service": "onlyfans",
    //             "title": "silly lil dancing videos i did ☺️",
    //             "content": "silly lil dancing videos i did ☺️",
    //             "embed": {},
    //             "shared_file": false,
    //             "added": "2024-05-26T03:04:55.971848",
    //             "published": "2024-05-21T17:19:18",
    //             "edited": null,
    //             "file": {
    //               "name": "0hpxn0h31vg5siq0xtqsl_source.mp4",
    //               "path": "/7d/1c/7d1c604743b8bf30c7c5a260ac487ae1dea4ef133cc767ed0c5989c981d56823.mp4"
    //             },
    //             "attachments": [
    //               {
    //                 "name": "0hpxn3cnhcj83v5zikmko_source.mp4",
    //                 "path": "/d0/06/d006cabcf8c68eb15b028b11736b6f902c6205bf3034f6c3f43b906126bc8c7f.mp4"
    //               }
    //             ],
    //             "poll": null,
    //             "captions": null,
    //             "tags": null,
    //             "next": "1075685308",
    //             "prev": "1082132822",
    //           }
    //         ]
    //       ]
    //     },
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct FavoritesPost {
    pub id: String,
    pub user: String,
    pub service: String,
    pub title: Option<String>,
    pub published: Option<String>,
    pub file: Option<PostFile>,
    pub attachments: Vec<PostFile>,
}

impl Post for FavoritesPost {
    fn files(&mut self) -> Vec<PostFile> {
        let mut files = Vec::new();
        if let Some(mut file) = self.file.clone() {
            file.service_override = Some(self.service.clone());
            file.user_override = Some(self.user.clone());
            files.push(file);
        }
        for mut attachment in self.attachments.clone() {
            attachment.service_override = Some(self.service.clone());
            attachment.user_override = Some(self.user.clone());
            files.push(attachment);
        }
        files.retain(PostFile::has_path);
        files
    }
}

pub async fn favorites_posts(offset: usize, domain: &str) -> Result<Vec<FavoritesPost>, ApiError> {
    fetch(&format!("https://{}/api/v1/account/favorites?type=post&o={offset}", domain)).await
}


#[derive(Debug, Clone, Deserialize)]
pub struct FavoriteCreator {
    pub id: String,
    pub service: String,
}

pub async fn favorites_creators(domain: &str) -> Result<Vec<FavoriteCreator>, ApiError> {
    fetch(&format!("https://{}/api/v1/account/favorites?type=artist", domain)).await
}
