use crate::{ cli::ARGUMENTS, http::CLIENT };
use anyhow::{ Context, Result, format_err };
use regex::{ Captures, Regex };
use serde::Deserialize;
use std::{
    collections::HashSet,
    fmt::{ self, Display, Formatter, Write },
    fs::File,
    io::{ BufRead, BufReader, Read },
    path::PathBuf,
    sync::LazyLock,
};
use strum_macros::{ Display, EnumString };

pub enum Target {
    Creator {
        service: Service,
        user: String,
        subtype: SubType,
        path: PathBuf,
        archive: HashSet<String>,
        archive_path: PathBuf,
    },
    Discord {
        server: String,
        channel: Option<String>,
        offset: Option<usize>,
        path: PathBuf,
        archive: HashSet<String>,
        archive_path: PathBuf,
    },
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Target::Creator { service, user, subtype, .. } => {
                let _ = write!(f, "{service}/{user}");

                if let SubType::Post(p) = subtype {
                    let _ = write!(f, "/{p}");
                }
            }
            Target::Discord { server, channel, .. } => {
                let _ = write!(f, "discord/{server}");

                if let Some(chan) = channel {
                    let _ = write!(f, "/{chan}");
                }
            }
        }

        Ok(())
    }
}

#[derive(PartialEq, Eq, Hash)]
pub enum SubType {
    PageOffset(usize),
    Post(String),
    None,
}

#[derive(Deserialize)]
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

static RE_DISCORD_PAGE: LazyRegex = LazyLock::new(|| {
    Regex::new(
        r"^(?:https://)?kemono\.(?:su|cr|party)/discord/server/(?<server>[0-9]{17,19})(/(?<channel>[0-9]{17,19}))?(\?o=(?<offset>\d+))?$"
    ).unwrap()
});

async fn try_fetch_linked_accounts(service: Service, user: &str) -> Result<Vec<Info>> {
    let host = service.host();
    let service = service.as_static_str();

    let mut url = String::with_capacity(8 + host.len() + 8 + service.len() + 6 + user.len() + 8);
    let _ = write!(url, "https://{host}/api/v1/{service}/user/{user}/profile");

    let account = CLIENT.get(url).send().await?.json().await?;
    let mut accounts = Vec::with_capacity(4);
    accounts.push(account);

    let mut linked_accounts_url = String::with_capacity(
        8 + host.len() + 8 + service.len() + 6 + user.len() + 6
    );
    let _ = write!(linked_accounts_url, "https://{host}/api/v1/{service}/user/{user}/links");

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

    #[allow(clippy::too_many_lines)]
    async fn try_from_url(url: &str) -> Result<Vec<Self>> {
        let url = url.strip_suffix('/').unwrap_or(url);

        let capture = |re: &Regex| re.captures(url).expect("get captures");
        let extract = |caps: &Captures, name: &str| caps.name(name).map(|m| m.as_str().to_string());
        let extract_unwrap = |caps: &Captures, name: &str|
            extract(caps, name).expect("extract values from captures");
        let service_user = |caps: &Captures| {
            if let Ok(service) = extract_unwrap(caps, "service").parse::<Service>() {
                Ok((service, extract_unwrap(caps, "user")))
            } else {
                Err(format_err!("Found invalid Service name when parsing URL"))
            }
        };
        let server_channel = |caps: &Captures| {
            (extract_unwrap(caps, "server"), extract(caps, "channel"))
        };
        let make_paths = |service, user_or_server: &str| {
            (make_pathbuf(service, user_or_server), make_archive_pathbuf(service, user_or_server))
        };

        if RE_LINKED.is_match(url) {
            let caps = capture(&RE_LINKED);

            let (service, user) = service_user(&caps)?;
            let linked = try_fetch_linked_accounts(service, &user).await?;

            let mut targets = Vec::new();

            for info in linked {
                let mut target = if info.service == "discord" {
                    let (path, archive_path) = make_paths(Service::Discord, &info.id);

                    Target::Discord {
                        server: info.id,
                        channel: None,
                        offset: None,
                        path,
                        archive: HashSet::new(),
                        archive_path,
                    }
                } else {
                    let (service, user) = (info.service.parse()?, info.id);
                    let (path, archive_path) = make_paths(service, &user);

                    Target::Creator {
                        service,
                        user,
                        subtype: SubType::None,
                        path,
                        archive: HashSet::new(),
                        archive_path,
                    }
                };

                if ARGUMENTS.download_archive {
                    target.try_read_archive()?;
                }

                targets.push(target);
            }

            return Ok(targets);
        }

        let archive = HashSet::new();

        let mut target = if RE_CREATOR.is_match(url) {
            let caps = capture(&RE_CREATOR);

            let (service, user) = service_user(&caps)?;
            let (path, archive_path) = make_paths(service, &user);

            Target::Creator {
                service,
                user,
                subtype: SubType::None,
                path,
                archive,
                archive_path,
            }
        } else if RE_PAGE.is_match(url) {
            let caps = capture(&RE_PAGE);

            let (service, user) = service_user(&caps)?;
            let (path, archive_path) = make_paths(service, &user);

            Target::Creator {
                service,
                user,
                subtype: SubType::PageOffset(extract_unwrap(&caps, "offset").parse()?),
                path,
                archive,
                archive_path,
            }
        } else if RE_POST.is_match(url) {
            let caps = capture(&RE_POST);

            let (service, user) = service_user(&caps)?;
            let (path, archive_path) = make_paths(service, &user);

            Target::Creator {
                service,
                user,
                subtype: SubType::Post(extract_unwrap(&caps, "post")),
                path,
                archive,
                archive_path,
            }
        } else if RE_DISCORD.is_match(url) {
            let caps = capture(&RE_DISCORD);

            let (server, channel) = server_channel(&caps);
            let (path, archive_path) = make_paths(Service::Discord, &server);

            Target::Discord {
                server,
                channel,
                offset: None,
                path,
                archive,
                archive_path,
            }
        } else if RE_DISCORD_PAGE.is_match(url) {
            let caps = capture(&RE_DISCORD_PAGE);

            let (server, channel) = server_channel(&caps);
            let (path, archive_path) = make_paths(Service::Discord, &server);

            Target::Discord {
                server,
                channel,
                offset: {
                    let offset = extract_unwrap(&caps, "offset").parse()?;
                    if offset % 150 == 0 {
                        Some(offset)
                    } else {
                        return Err(format_err!("Invalid URL: {url}"));
                    }
                },
                path,
                archive,
                archive_path,
            }
        } else {
            return Err(format_err!("Invalid URL: {url}"));
        };

        if ARGUMENTS.download_archive {
            target.try_read_archive()?;
        }

        Ok(Vec::from_iter([target]))
    }

    pub fn try_read_archive(&mut self) -> Result<()> {
        let mut archive = File::options()
            .read(true)
            .append(true)
            .create(true)
            .truncate(false)
            .open(self.as_archive_pathbuf())
            .with_context(|| {
                let file = self.to_string();
                let mut buf = String::with_capacity(32 + file.len());
                let _ = write!(buf, "Failed to open archive file for {file}");
                buf
            })?;

        let mut arc_buf = String::new();

        archive.read_to_string(&mut arc_buf)?;

        self.add_hashes(arc_buf.lines().map(ToString::to_string).collect());

        Ok(())
    }

    fn add_hashes(&mut self, hashes: HashSet<String>) {
        match self {
            Target::Creator { archive, .. } | Target::Discord { archive, .. } => {
                archive.extend(hashes);
            }
        }
    }

    pub fn archive(&self) -> &HashSet<String> {
        match self {
            Target::Creator { archive, .. } | Target::Discord { archive, .. } => archive,
        }
    }

    pub fn as_pathbuf(&self) -> &PathBuf {
        match self {
            Target::Creator { path, .. } | Target::Discord { path, .. } => path,
        }
    }

    pub fn as_archive_pathbuf(&self) -> &PathBuf {
        match self {
            Target::Creator { archive_path, .. } | Target::Discord { archive_path, .. } =>
                archive_path,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, EnumString, Display)]
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
    pub fn as_static_str(self) -> &'static str {
        #[allow(clippy::enum_glob_use)]
        use Service::*;
        match self {
            Afdian => "afdian",
            Boosty => "boosty",
            CandFans => "candfans",
            Discord => "discord",
            DlSite => "dlsite",
            Fanbox => "fanbox",
            Fansly => "fansly",
            Fantia => "fantia",
            Gumroad => "gumroad",
            OnlyFans => "onlyfans",
            Patreon => "patreon",
            SubscribeStar => "subscribestar",
        }
    }

    pub fn host(self) -> &'static str {
        #[allow(clippy::enum_glob_use)]
        use Service::*;
        match self {
            CandFans | Fansly | OnlyFans => "coomer.st",
            _ => "kemono.cr",
        }
    }
}

fn make_pathbuf(service: Service, user: &str) -> PathBuf {
    PathBuf::from_iter([&ARGUMENTS.output_path, service.as_static_str(), user])
}

fn make_archive_pathbuf(service: Service, user: &str) -> PathBuf {
    PathBuf::from_iter([
        &ARGUMENTS.output_path,
        "db",
        &({
            let service = service.as_static_str();

            let mut file_name = String::with_capacity(service.len() + 1 + user.len() + 4);
            let _ = write!(file_name, "{service}+{user}.txt");

            file_name
        }),
    ])
}
