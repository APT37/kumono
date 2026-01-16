use crate::{ cli::ARGUMENTS, http::CLIENT };
use anyhow::{ Context, Result, anyhow };
use regex::{ Captures, Regex };
use serde::Deserialize;
use std::{
    fmt::{ self, Display, Formatter },
    fs::File,
    io::{ BufRead, BufReader, Read },
    path::PathBuf,
    sync::LazyLock,
};
use strum_macros::{ Display, EnumString };

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    Creator {
        service: Service,
        user: String,
        subtype: SubType,
        archive: Vec<String>,
    },
    Discord {
        server: String,
        channel: Option<String>,
        archive: Vec<String>,
    },
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Target::Creator { service, user, subtype, .. } =>
                format!(
                    "{service}/{user}{post}",
                    post = if let SubType::Post(p) = subtype {
                        format!("/{p}")
                    } else {
                        String::new()
                    }
                ),
            Target::Discord { server, channel, .. } =>
                format!(
                    "discord/{server}{channel}",
                    channel = channel.as_ref().map_or(String::new(), |c| format!("/{c}"))
                ),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubType {
    PageOffset(usize),
    Post(String),
    None,
}

#[derive(Debug, Clone, Deserialize)]
struct Info {
    id: String, // "5564244",
    // name: String, // "theobrobine",
    service: String, // "patreon",
    // indexed: String, // "2020-09-30T06:13:38.348472",
    // updated: String, // "2025-05-30T14:07:16.596525",
    // public_id: Option<String>, // "theobrobine",
    // relation_id: Option<u64>, // 8,
    // has_chats: Option<bool>, // false
}

type LazyRegex = LazyLock<Regex>;

static RE_LINKED: LazyRegex = LazyLock::new(|| {
    Regex::new(
        r"^(?:https://)?(?:coomer\.(?:su|st|party)|kemono\.(?:su|cr|party))/(?<service>[a-z]+)/user/(?<user>[a-z|A-Z|0-9|\-|_|\.]+)/links$"
    ).unwrap()
});

static RE_CREATOR: LazyRegex = LazyLock::new(|| {
    Regex::new(
        r"^(?:https://)?(?:coomer\.(?:su|st|party)|kemono\.(?:su|cr|party))/(?<service>[a-z]+)/user/(?<user>[a-z|A-Z|0-9|\-|_|\.]+)$"
    ).unwrap()
});

static RE_PAGE: LazyRegex = LazyLock::new(|| {
    Regex::new(
        r"^(?:https://)?(?:coomer\.(?:su|st|party)|kemono\.(?:su|cr|party))/(?<service>[a-z]+)/user/(?<user>[a-z|A-Z|0-9|\-|_|\.]+)\?o=(?<offset>(0|50|[1-9]+(0|5)0))$"
    ).unwrap()
});

static RE_POST: LazyRegex = LazyLock::new(|| {
    Regex::new(
        r"^(?:https://)?(?:coomer\.(?:su|st|party)|kemono\.(?:su|cr|party))/(?<service>[a-z]+)/user/(?<user>[a-z|A-Z|0-9|\-|_|\.]+)/post/(?<post>[a-z|A-Z|0-9|\-|_|\.]+)$"
    ).unwrap()
});

static RE_DISCORD: LazyRegex = LazyLock::new(|| {
    Regex::new(
        r"^(?:https://)?kemono\.(?:su|cr|party)/discord/server/(?<server>[0-9]{17,19})(/(?<channel>[0-9]{17,19}))?$"
    ).unwrap()
});

async fn try_fetch_linked_accounts(service: &Service, user: &str) -> Result<Vec<Info>> {
    let mut accounts = Vec::with_capacity(4);

    let url = format!("https://{site}/api/v1/{service}/user/{user}/profile", site = service.site());
    let account = CLIENT.get(url).send().await?.json().await?;
    accounts.push(account);

    let linked_accounts_url = format!(
        "https://{site}/api/v1/{service}/user/{user}/links",
        site = service.site()
    );
    let mut linked_accounts = CLIENT.get(linked_accounts_url).send().await?.json().await?;
    accounts.append(&mut linked_accounts);

    Ok(accounts)
}

impl Target {
    pub fn as_service(&self) -> Service {
        match *self {
            Target::Creator { service, .. } => service,
            Target::Discord { .. } => Service::Discord,
        }
    }

    pub async fn try_parse_file() -> Result<Vec<Target>> {
        let mut targets = Vec::new();

        if let Some(files) = &ARGUMENTS.input_files {
            for path in files {
                for line in BufReader::new(File::open(path)?).lines() {
                    match Target::try_from_url(&line?).await {
                        Ok(mut target) => targets.append(&mut target),
                        Err(err) => eprintln!("{err}"),
                    }
                }
            }
        }

        Ok(targets)
    }

    pub async fn parse_args() -> Vec<Target> {
        let mut targets = Vec::with_capacity(ARGUMENTS.urls.len());

        for url in &ARGUMENTS.urls {
            match Target::try_from_url(url).await {
                Ok(mut target) => targets.append(&mut target),
                Err(err) => eprintln!("{err}"),
            }
        }

        targets
    }

    async fn try_from_url(url: &str) -> Result<Vec<Self>> {
        let url = url.strip_suffix('/').unwrap_or(url);

        let capture = |re: &Regex| re.captures(url).expect("get captures");
        let extract = |caps: &Captures, name: &str| caps.name(name).map(|m| m.as_str().to_string());
        let extract_unwrap = |caps: &Captures, name: &str| {
            extract(caps, name).expect("extract values from captures")
        };

        if RE_LINKED.is_match(url) {
            let caps = capture(&RE_LINKED);

            let linked = try_fetch_linked_accounts(
                &extract_unwrap(&caps, "service").parse()?,
                &extract_unwrap(&caps, "user")
            ).await?;

            let mut targets = Vec::new();

            for info in linked {
                let mut target = if info.service == "discord" {
                    Target::Discord {
                        server: info.id,
                        channel: None,
                        archive: Vec::new(),
                    }
                } else {
                    Target::Creator {
                        service: info.service.parse()?,
                        user: info.id,
                        subtype: SubType::None,
                        archive: Vec::new(),
                    }
                };

                if ARGUMENTS.download_archive {
                    target.try_read_archive()?;
                }

                targets.push(target);
            }

            return Ok(targets);
        }

        let archive = Vec::new();

        let mut target = if RE_CREATOR.is_match(url) {
            let caps = capture(&RE_CREATOR);
            Target::Creator {
                service: extract_unwrap(&caps, "service").parse()?,
                user: extract_unwrap(&caps, "user"),
                subtype: SubType::None,
                archive,
            }
        } else if RE_PAGE.is_match(url) {
            let caps = capture(&RE_PAGE);
            Target::Creator {
                service: extract_unwrap(&caps, "service").parse()?,
                user: extract_unwrap(&caps, "user"),
                subtype: SubType::PageOffset(extract_unwrap(&caps, "offset").parse()?),
                archive,
            }
        } else if RE_POST.is_match(url) {
            let caps = capture(&RE_POST);
            Target::Creator {
                service: extract_unwrap(&caps, "service").parse()?,
                user: extract_unwrap(&caps, "user"),
                subtype: SubType::Post(extract_unwrap(&caps, "post")),
                archive,
            }
        } else if RE_DISCORD.is_match(url) {
            let caps = capture(&RE_DISCORD);
            Target::Discord {
                server: extract_unwrap(&caps, "server"),
                channel: extract(&caps, "channel"),
                archive,
            }
        } else {
            return Err(anyhow!("Invalid URL: {url}"));
        };

        if ARGUMENTS.download_archive {
            target.try_read_archive()?;
        }

        Ok(Vec::from_iter([target]))
    }

    fn user(&self) -> &str {
        match self {
            Target::Creator { user, .. } => user,
            Target::Discord { server, .. } => server,
        }
    }

    pub fn try_read_archive(&mut self) -> Result<()> {
        let mut archive = File::options()
            .read(true)
            .append(true)
            .create(true)
            .truncate(false)
            .open(self.to_archive_pathbuf())
            .with_context(|| format!("Failed to open archive file for {self}"))?;

        let mut buffer = String::new();

        archive.read_to_string(&mut buffer)?;

        self.add_hashes(buffer.lines().map(ToString::to_string).collect());

        Ok(())
    }

    fn add_hashes(&mut self, mut hashes: Vec<String>) {
        match self {
            Target::Creator { archive, .. } | Target::Discord { archive, .. } => {
                archive.append(&mut hashes);
            }
        }
    }

    pub fn archive(&self) -> &Vec<String> {
        match self {
            Target::Creator { archive, .. } | Target::Discord { archive, .. } => archive,
        }
    }

    pub fn to_archive_pathbuf(&self) -> PathBuf {
        PathBuf::from_iter([
            &ARGUMENTS.output_path,
            "db",
            &format!("{service}+{user}.txt", service = self.as_service(), user = self.user()),
        ])
    }

    pub fn to_pathbuf(&self, file: Option<&str>) -> PathBuf {
        PathBuf::from_iter([
            &ARGUMENTS.output_path,
            &self.as_service().to_string(),
            self.user(),
            file.unwrap_or_default(),
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumString, Display)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum Service {
    Afdian,
    Boosty,
    CandFans,
    Discord,
    DlSite,
    Fanbox,
    Fansly,
    Fantia,
    Gumroad,
    OnlyFans,
    Patreon,
    SubscribeStar,
}

impl Service {
    pub fn site(self) -> &'static str {
        #[allow(clippy::enum_glob_use)]
        use Service::*;
        match self {
            CandFans | Fansly | OnlyFans => "coomer.st",
            _ => "kemono.cr",
        }
    }
}
