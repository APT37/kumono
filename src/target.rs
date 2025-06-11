use crate::cli::ARGS;
use anyhow::{ bail, Result };
use std::{ path::PathBuf, process::exit, sync::LazyLock };
use regex::{ Match, Regex };

pub static TARGET: LazyLock<Target> = LazyLock::new(||
    Target::from_url(&ARGS.url).unwrap_or_else(|err| {
        eprintln!("{err}");
        exit(1)
    })
);

#[derive(Debug, Clone)]
pub struct Target {
    pub service: String,
    pub user: String,
    pub post: Option<String>,
}

static RE_DEFAULT: LazyLock<Regex> = LazyLock::new(||
    Regex::new(
        r"^(https://)?(coomer|kemono)\.su/(?<service>afdian|boosty|candfans|dlsite|fanbox|fansly|fantia|gumroad|onlyfans|patreon|subscribestar)/user/(?<user>[a-z|A-Z|0-9|\-|_|\.]+)(/post/(?<post>[a-z|A-Z|0-9|\-|_|\.]+))?"
    ).unwrap()
);

static RE_DISCORD: LazyLock<Regex> = LazyLock::new(||
    Regex::new(
        r"^(https://)?kemono\.su/discord/server/(?<server>[0-9]{19})(/(?<channel>[0-9]{19}))?"
    ).unwrap()
);

impl Target {
    fn new(service: String, user: String, post: Option<Match>) -> Self {
        Target {
            service,
            user,
            post: post.map(|p| p.as_str().to_string()),
        }
    }

    fn from_url(url: &str) -> Result<Self> {
        if let Some(caps) = RE_DEFAULT.captures(url) {
            match (&caps.name("service"), &caps.name("user")) {
                (None, _) => bail!("Invalid service in URL: {url}"),
                (Some(_), None) => bail!("Invalid user in URL: {url}"),
                (Some(s), Some(u)) =>
                    Ok(
                        Target::new(
                            s.as_str().to_string(),
                            u.as_str().to_string(),
                            caps.name("post")
                        )
                    ),
            }
        } else if let Some(caps) = RE_DISCORD.captures(&ARGS.url) {
            if let Some(server) = &caps.name("server") {
                Ok(
                    Target::new(
                        "discord".to_string(),
                        server.as_str().to_string(),
                        caps.name("channel")
                    )
                )
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
        PathBuf::from_iter([&self.service.to_string(), &self.user])
    }

    pub fn to_pathbuf_with_file(&self, file: impl AsRef<str>) -> PathBuf {
        PathBuf::from_iter([&self.service.to_string(), &self.user, file.as_ref()])
    }
}
