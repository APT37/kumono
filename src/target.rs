use crate::{ cli::ARGS, http::CLIENT };
use anyhow::{ Result, bail };
use regex::{ Match, Regex };
use serde::Deserialize;
use std::{ path::PathBuf, process::exit, sync::LazyLock };

pub static TARGETS: LazyLock<Vec<Target>> = LazyLock::new(|| {
    let mut targets = Vec::new();

    for url in &ARGS.urls {
        match Target::from_url(url) {
            Ok(target) => targets.push(target),
            Err(err) => eprintln!("{err}"),
        }
    }

    if targets.is_empty() {
        eprintln!("No valid target URLs found.");
        exit(1);
    }

    targets.sort();
    targets.dedup();
    targets
});

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Target {
    pub service: String,
    pub user: String,
    pub page: Option<String>,
    pub post: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum User {
    #[allow(unused)] Info(Info),
    Error {
        error: String,
    },
}

#[allow(unused)]
#[derive(Debug, Clone, Deserialize)]
struct Info {
    id: String, // "5564244",
    name: String, // "theobrobine",
    service: String, // "patreon",
    indexed: String, // "2020-09-30T06:13:38.348472",
    updated: String, // "2025-05-30T14:07:16.596525",
    public_id: String, // "theobrobine",
    relation_id: Option<u64>, // 8,
    has_chats: Option<bool>, // false
}

static RE_DEFAULT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(https://)?(coomer|kemono)\.su/(?<service>afdian|boosty|candfans|dlsite|fanbox|fansly|fantia|gumroad|onlyfans|patreon|subscribestar)/user/(?<user>[a-z|A-Z|0-9|\-|_|\.]+)((\?o=(?<page>([1-9]+[0|5]+|5)?0))|(/post/(?<post>[a-z|A-Z|0-9|\-|_|\.]+)))?$"
    ).unwrap()
});

static RE_DISCORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(https://)?kemono\.su/discord/server/(?<server>[0-9]{17,19})(/(?<channel>[0-9]{17,19}))?$"
    ).unwrap()
});

impl Target {
    fn new(service: &str, user: &str, page: Option<Match>, post: Option<Match>) -> Self {
        let m = |m: Match| m.as_str().to_string();

        Target {
            service: service.to_string(),
            user: user.to_string(),
            page: page.map(m),
            post: post.map(m),
        }
    }

    pub async fn exists(&self) -> Result<()> {
        if self.service != "discord" {
            let url = format!(
                "https://{}.su/api/v1/{}/user/{}/profile",
                self.site(),
                self.service,
                self.user
            );

            match CLIENT.get(url).send().await?.json().await? {
                User::Info(_) => {}
                User::Error { error: err } => bail!("{err}"),
            }
        }

        Ok(())
    }

    fn from_url(url: &str) -> Result<Self> {
        if let Some(caps) = RE_DEFAULT.captures(url) {
            match (&caps.name("service"), &caps.name("user")) {
                (None, _) => bail!("Invalid service in URL: {url}"),
                (Some(_), None) => bail!("Invalid user in URL: {url}"),
                (Some(s), Some(u)) =>
                    Ok(Target::new(s.as_str(), u.as_str(), caps.name("page"), caps.name("post"))),
            }
        } else if let Some(caps) = RE_DISCORD.captures(url) {
            if let Some(server) = &caps.name("server") {
                Ok(Target::new("discord", server.as_str(), None, caps.name("channel")))
            } else {
                bail!("Invalid Discord server in URL: {url}")
            }
        } else {
            bail!("Invalid URL: {url}");
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

            _ => unimplemented!("Unknown Service"),
        }
    }

    pub fn to_pathbuf(&self, file: Option<&str>) -> PathBuf {
        let mut path = PathBuf::from_iter([&ARGS.output_path, &self.service, &self.user]);

        if let Some(file) = file {
            path.push(file);
        }

        path
    }
}
