use crate::{ cli::ARGS, http::CLIENT };
use anyhow::{ Result, bail };
use regex::{ Match, Regex };
use serde::Deserialize;
use std::{ path::PathBuf, process::exit, sync::LazyLock };

pub static TARGET: LazyLock<Target> = LazyLock::new(|| {
    Target::from_url(&ARGS.url).unwrap_or_else(|err| {
        eprintln!("{err}");
        exit(1)
    })
});

#[derive(Debug, Clone)]
pub struct Target {
    pub service: String,
    pub user: String,
    pub post: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum User {
    #[allow(dead_code)] Info(Info),

    Error {
        error: String,
    },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct Info {
    id: String, // "5564244",
    name: String, // "theobrobine",
    service: String, // "patreon",
    indexed: String, // "2020-09-30T06:13:38.348472",
    updated: String, // "2025-05-30T14:07:16.596525",
    public_id: String, // "theobrobine",
    relation_id: Option<u64>, // 8,
    has_chats: bool, // false
}

static RE_DEFAULT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(https://)?(coomer|kemono)\.su/(?<service>afdian|boosty|candfans|dlsite|fanbox|fansly|fantia|gumroad|onlyfans|patreon|subscribestar)/user/(?<user>[a-z|A-Z|0-9|\-|_|\.]+)(/post/(?<post>[a-z|A-Z|0-9|\-|_|\.]+))?"
    ).unwrap()
});

static RE_DISCORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(https://)?kemono\.su/discord/server/(?<server>[0-9]{17,19})(/(?<channel>[0-9]{17,19}))?"
    ).unwrap()
});

impl Target {
    fn new(service: &str, user: &str, post: Option<Match>) -> Self {
        Target {
            service: service.to_string(),
            user: user.to_string(),
            post: post.map(|p| p.as_str().to_string()),
        }
    }

    pub async fn exists(&self) -> Result<()> {
        if self.service == "discord" {
            return Ok(());
        }

        let url = format!(
            "https://{}.su/api/v1/{}/user/{}/profile",
            self.site(),
            self.service,
            self.user
        );

        match CLIENT.get(url).send().await?.json().await? {
            User::Info(_) => Ok(()),
            User::Error { error: err } => bail!("{err}"),
        }
    }

    fn from_url(url: &str) -> Result<Self> {
        if let Some(caps) = RE_DEFAULT.captures(url) {
            match (&caps.name("service"), &caps.name("user")) {
                (None, _) => bail!("Invalid service in URL: {url}"),
                (Some(_), None) => bail!("Invalid user in URL: {url}"),
                (Some(s), Some(u)) => Ok(Target::new(s.as_str(), u.as_str(), caps.name("post"))),
            }
        } else if let Some(caps) = RE_DISCORD.captures(&ARGS.url) {
            if let Some(server) = &caps.name("server") {
                Ok(Target::new("discord", server.as_str(), caps.name("channel")))
            } else {
                bail!("Invalid Discord server in URL: {url}")
            }
        } else {
            bail!("invalid URL: {url}");
        }
    }

    pub fn site(&self) -> &'static str {
        match self.service.as_str() {
            "candfans" | "fansly" | "onlyfans" => "coomer",

            | "afdian"
            | "boosty"
            | "discord"
            | "dlsite"
            | "fanbox"
            | "fantia"
            | "gumroad"
            | "patreon"
            | "subscribestar" => "kemono",

            _ => unimplemented!("unknown service"),
        }
    }

    pub fn to_pathbuf(&self) -> PathBuf {
        PathBuf::from_iter([&self.service, &self.user])
    }

    pub fn to_pathbuf_with_file(&self, file: impl AsRef<str>) -> PathBuf {
        PathBuf::from_iter([&self.service, &self.user, file.as_ref()])
    }
}
