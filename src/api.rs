use crate::{ file::PostFile, http::CLIENT, target::Target, cli::ARGS };
use anyhow::{ bail, Result };
use reqwest::StatusCode;
use serde::{ de::DeserializeOwned, Deserialize };
use thiserror::Error;
use tokio::time::{ Duration, sleep };

const API_DELAY: Duration = Duration::from_millis(100);

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
                match *status {
                    StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS =>
                        wait_or_bail(retries, ARGS.rate_limit_backoff, status.as_str()).await?,
                    s => wait_or_bail(retries, ARGS.retry_delay, s.as_str()).await?,
                }
            ApiError::Parser(err) => wait_or_bail(retries, ARGS.retry_delay, err).await?,
        }

        Ok(())
    }
}

async fn fetch<T: DeserializeOwned>(url: &str) -> Result<T, ApiError> {
    let res = CLIENT.get(url)
        .send().await
        .map_err(|err| ApiError::Connect(err.to_string()))?;

    let status = res.status();
    if status != StatusCode::OK {
        return Err(ApiError::Status(status));
    }

    res.json().await.map_err(|err| ApiError::Parser(err.to_string()))
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
    //             "prev": "1082132822"
    //           }
    //         ]
    //       ]
    //     }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct SinglePostInner {
    id: String, // "1080444052",
    user: String, // "belledelphine",
    service: String, // "onlyfans",
    title: String, // "silly lil dancing videos i did ☺️",
    content: String, // "silly lil dancing videos i did ☺️",
    //   embed: {}, // again, we might need to fetch this too, IF any posts happen to have it
    //   shared_file: false,
    //   added: "2024-05-26T03:04:55.971848",
    //   published: "2024-05-21T17:19:18",
    //   edited: null,
    file: Option<PostFile>,
    attachments: Vec<PostFile>,
    //   "poll": null,
    //   "captions": null,
    //   "tags": null,
    //   "next": "1075685308",
    //   "prev": "1082132822"
}

impl Post for SinglePost {
    fn files(&mut self) -> Vec<PostFile> {
        let mut files: Vec<PostFile> = Vec::new();
        if let Some(file) = self.post.file.as_ref() {
            files.push(file.clone());
        }
        files.append(&mut self.post.attachments);
        files.retain(|pf| pf.path.is_some());
        files
    }
}

pub async fn page(target: &Target, user: &str, offset: usize) -> Result<Vec<PagePost>, ApiError> {
    sleep(API_DELAY).await;

    let url = format!(
        "https://{site}.su/api/v1/{service}/user/{user}?o={offset}",
        site = target.service().site(),
        service = target.service()
    );

    let res = CLIENT.get(url)
        .send().await
        .map_err(|err| ApiError::Connect(err.to_string()))?;

    let status = res.status();
    if status != StatusCode::OK {
        return Err(ApiError::Status(status));
    }

    res.json().await.map_err(|err| ApiError::Parser(err.to_string()))
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PagePost {
    id: String, // "132018745"
    user: String, // "5564244"
    service: String, // "patreon",
    title: String, // "[wip,short,color,gif,png] futanari hasumi paizuri",
    content: String, // "<p>I will also post an update on the progress of the One Piece animation this month.</p>",
    // "embed": {}, // find out if posts ever have this
    // "shared_file": false,
    // "added": "2025-06-24T03:04:27.561458",
    // "published": "2025-06-22T08:34:11",
    // "edited": "2025-06-22T12:20:40",
    file: Option<PostFile>,
    attachments: Vec<PostFile>,
    // "poll": null,
    // "captions": null,
    // "tags": "{animation,futa,futanari,gif,hasumi,paizuri,png,wip}"
}

impl Post for PagePost {
    fn files(&mut self) -> Vec<PostFile> {
        let mut files: Vec<PostFile> = Vec::new();
        if let Some(file) = self.file.as_ref() {
            files.push(file.clone());
        }
        files.append(&mut self.attachments);
        files.retain(|pf| pf.path.is_some());
        files
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscordPost {
    // "id": "1196530932195266712",
    // "author": {
    //   "id": "1194345049450881087",
    //   "flags": 0,
    //   "avatar": null,
    //   "banner": null,
    //   "username": "myuto12345",
    //   "global_name": "myuto",
    //   "accent_color": null,
    //   "banner_color": null,
    //   "premium_type": 2,
    //   "public_flags": 0,
    //   "discriminator": "0",
    //   "avatar_decoration_data": null
    // },
    // // "server": "1196504962411282491",
    // "channel": "1196521501059469463",
    // content: String, // "多くの投稿について未成年に見えるとして削除されてしまったので移行しました\nThe character was determined to be a minor and was removed.\n该角色被确定为未成年人，已被删除。",
    // "added": "2024-04-03T00:16:27.719127",
    // "published": "2024-01-15T19:06:44.705000",
    // "edited": null,
    // "embeds": [],
    // "mentions": [],
    attachments: Vec<PostFile>,
    // "seq": 0
}

impl Post for DiscordPost {
    fn files(&mut self) -> Vec<PostFile> {
        let mut files: Vec<PostFile> = Vec::new();
        files.append(&mut self.attachments);
        files.retain(|pf| pf.path.is_some());
        files
    }
}

pub async fn discord_page(channel: &str, offset: usize) -> Result<Vec<DiscordPost>, ApiError> {
    sleep(API_DELAY).await;

    let url = format!("https://kemono.su/api/v1/discord/channel/{channel}?o={offset}");

    let res = CLIENT.get(url)
        .send().await
        .map_err(|err| ApiError::Connect(err.to_string()))?;

    let status = res.status();
    if status != StatusCode::OK {
        return Err(ApiError::Status(status));
    }

    res.json().await.map_err(|err| ApiError::Parser(err.to_string()))
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscordChannel {
    pub id: String, // "455285536341491716",
    // name: String, // "news"
}

pub async fn discord_server(server: &str) -> Result<Vec<DiscordChannel>, ApiError> {
    fetch(&format!("https://kemono.su/api/v1/discord/channel/lookup/{server}")).await
}
