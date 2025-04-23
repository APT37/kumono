use crate::{ client::CLIENT, config::CONFIG, n_fmt };
use anyhow::Result;
use futures_util::StreamExt;
use log::{ debug, error, info, warn };
use reqwest::StatusCode;
use serde::Deserialize;
use size::Size;
use std::{ path::PathBuf, process };
use tokio::{ fs, io::AsyncWriteExt, time::sleep };

pub struct Profile<'a> {
    service: &'a str,
    creator: &'a str,
    pub posts: Vec<Post>,
    pub files: Vec<TargetFile>,
}

impl<'a> Profile<'a> {
    pub async fn new(service: &'a str, creator: &'a str) -> Result<Self> {
        let mut profile = Self {
            service,
            creator,
            posts: vec![],
            files: vec![],
        };

        profile.init_posts().await?;
        profile.init_files();

        info!(
            "found {} posts, containing {} files",
            n_fmt(profile.posts.len()),
            n_fmt(profile.files.len())
        );

        Ok(profile)
    }

    pub async fn init_posts(&mut self) -> Result<()> {
        info!("fetching posts for {}/{}", self.service, self.creator);

        let mut offset = 0;

        loop {
            debug!("fetching posts for {}/{} with offset {offset}", self.service, self.creator);

            let mut posts: Vec<Post>;

            let url = self.api_url_with_offset(offset);

            loop {
                sleep(CONFIG.api_delay()).await;

                let response = CLIENT.get(&url).send().await?;

                let status = response.status();

                if status == StatusCode::OK {
                    posts = response.json().await?;
                    break;
                } else if status == StatusCode::TOO_MANY_REQUESTS {
                    warn!(
                        "hit rate-limiting at offset {offset}, sleeping for {}",
                        pretty_duration::pretty_duration(&CONFIG.api_backoff(), None)
                    );
                    sleep(CONFIG.api_backoff()).await;
                } else {
                    error!("got unhandled status {status} when requesting {url}");
                    process::exit(1);
                }
            }

            if posts.is_empty() {
                break;
            }

            self.posts.append(&mut posts);

            offset += 50;
        }

        Ok(())
    }

    fn api_url_with_offset(&self, offset: u32) -> String {
        format!("https://coomer.su/api/v1/{}/user/{}?o={offset}", self.service, self.creator)
    }

    fn init_files(&mut self) {
        self.posts
            .clone()
            .into_iter()
            .for_each(|post| self.files.append(&mut post.files()));

        self.files.sort();
        self.files.dedup();
    }
}

#[derive(Deserialize, Clone)]
pub struct Post {
    // id: String,   // "1000537173"
    pub user: String, // "paigetheuwulord"
    // service: String,  // "onlyfans"
    // title: String,     // "What an ass"
    file: PostFile,
    attachments: Vec<PostFile>,
}

impl Post {
    pub fn files(mut self) -> Vec<TargetFile> {
        let mut files = vec![self.file];

        files.append(&mut self.attachments);

        files
            .into_iter()
            .filter_map(|f| {
                if let (Some(name), Some(path)) = (f.name, f.path) {
                    Some(TargetFile::new(&self.user, &name, &path))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Deserialize, Clone)]
struct PostFile {
    name: Option<String>, // "1242x2208_882b040faaac0e38fba20f4caadb2e59.jpg",
    path: Option<String>, // "/6e/6c/6e6cf84df44c1d091a2e36b6df77b098107c18831833de1e2e9c8207206f150b.jpg"
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetFile {
    name: String,
    url: String, // "https://coomer.su/data/6e/6c/6e6cf84df44c1d091a2e36b6df77b098107c18831833de1e2e9c8207206f150b.jpg"
    fs_path: PathBuf,
    fs_path_temp: PathBuf,
}

impl TargetFile {
    fn new(creator: &str, filename: &str, path: &str) -> Self {
        Self {
            name: filename.to_string(),

            url: format!("https://coomer.su/data{path}"),

            fs_path: PathBuf::from(format!("{creator}/{filename}")),
            fs_path_temp: PathBuf::from(format!("{creator}/{filename}.temp")),
        }
    }

    pub async fn download(&self) -> Result<Option<Size>> {
        if fs::try_exists(&self.fs_path).await? {
            info!("skipping {}", self.name);

            return Ok(None);
        }

        let size = self.size().await;

        info!("downloading {} ({size})", self.name);

        let mut stream = CLIENT.get(&self.url).send().await?.bytes_stream();

        let mut file = fs::File::create(&self.fs_path_temp).await?;

        while let Some(Ok(bytes)) = stream.next().await {
            file.write_all(&bytes).await?;
        }

        file.flush().await?;

        fs::rename(&self.fs_path_temp, &self.fs_path).await?;

        Ok(Some(size))
    }

    async fn size(&self) -> Size {
        if let Ok(res) = CLIENT.head(&self.url).send().await {
            if res.status().as_u16() == 200 {
                return Size::from_bytes(res.content_length().unwrap_or_default());
            }
        }

        Size::default()
    }
}
