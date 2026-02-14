use crate::{
    file::PostFile,
    http::CLIENT,
    post::{ self, DiscordChannel, DiscordPost, PagePost, Post, PostError, SinglePost },
    pretty::{ self, n_fmt },
    target::{ SubType, Target },
};
use anyhow::Result;
use indicatif::{ ProgressBar, ProgressStyle };
use serde::Deserialize;
use std::{ collections::HashSet, fmt::{ self, Display, Formatter, Write }, sync::Arc, thread };
use tokio::{ sync::mpsc::{ UnboundedReceiver, unbounded_channel }, time::{ Duration, sleep } };

#[derive(Deserialize)]
struct Creator {
    // id: usize, // "1024383"
    // name: String, // "sspr"
    // service: Service, // "fanbox"
    // indexed: String, // "2021-10-07T02:46:54.888210"
    // updated: String, // "2025-05-30T13:38:51.561232"
    // public_id: String, // "sspr"
    // relation_id: usize, // null
    post_count: usize, // 165
    // dm_count: usize, // 0
    // share_count: usize, // 0
    // chat_count: usize, // 0
}

pub struct Profile {
    target_id: usize,
    target: Arc<Target>,
    posts: Vec<Box<dyn Post>>,
    pub files: HashSet<Arc<PostFile>>,
}

impl Display for Profile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self.target {
            | Target::Creator { subtype: SubType::Post(_), .. }
            | Target::Discord { channel: None, .. } => {
                let _ = write!(
                    f,
                    "#{number}: {target} has {files}",
                    number = n_fmt(self.target_id),
                    target = self.target,
                    files = pretty::with_word(self.files.len(), "file")
                );
            }
            _ => {
                let cap = self.posts.capacity();

                let _ = write!(
                    f,
                    "#{number}: {target} has {posts}",
                    number = n_fmt(self.target_id),
                    target = self.target,
                    posts = pretty::with_word(cap, "post")
                );

                if cap > 0 {
                    let _ = match self.files.len() {
                        0 => write!(f, ", but no files"),
                        1 => write!(f, ", containing 1 file"),
                        n => write!(f, ", containing {} files", n_fmt(n)),
                    };
                }
            }
        }

        Ok(())
    }
}

fn page_progress(mut msg_rx: UnboundedReceiver<String>) {
    let bar = ProgressBar::new_spinner();

    bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {msg}").unwrap());

    bar.enable_steady_tick(Duration::from_millis(200));

    while let Some(msg) = msg_rx.blocking_recv() {
        bar.set_message(msg);
    }

    bar.finish();
}

impl Profile {
    pub async fn try_new(target: Arc<Target>, target_id: usize) -> Result<Self> {
        let post_count = if let Target::Creator { service, user, .. } = &*target {
            let host = service.host();
            let service = service.as_static_str();
            let mut url = String::with_capacity(
                8 + host.len() + 8 + service.len() + 6 + user.len() + 8
            );
            let _ = write!(url, "https://{host}/api/v1/{service}/user/{user}/profile");
            CLIENT.get(&url).send().await?.json::<Creator>().await?.post_count
        } else {
            0
        };

        let mut profile = Self {
            target_id,
            target: target.clone(),
            posts: Vec::with_capacity(post_count),
            files: HashSet::new(),
        };

        match &*target {
            Target::Creator { user, subtype, .. } => {
                profile.init_posts_standard(user, subtype).await?;
            }
            Target::Discord { server, channel, offset, .. } => {
                profile.init_posts_discord(server, channel, offset).await?;
            }
        }

        // wait for page progress bar to finish
        sleep(Duration::from_millis(1)).await;

        profile.init_files();

        eprintln!("{profile}");

        // discard posts, shrink vector
        profile.posts.clear();
        profile.posts.shrink_to_fit();

        Ok(profile)
    }

    async fn init_posts_standard(&mut self, user: &str, subtype: &SubType) -> Result<()> {
        if let SubType::Post(post) = subtype {
            let post: SinglePost = CLIENT.get({
                let host = self.target.as_service().host();
                let service = self.target.as_service().as_static_str();

                let mut url = String::with_capacity(
                    8 + host.len() + 8 + service.len() + 6 + user.len() + 6 + post.len()
                );
                let _ = write!(url, "https://{host}/api/v1/{service}/user/{user}/post/{post}");

                url
            })
                .send().await?
                .json().await?;

            self.posts.push(Box::new(post));
        } else {
            let (msg_tx, msg_rx) = unbounded_channel::<String>();

            thread::spawn(move || page_progress(msg_rx));

            let mut offset = if let SubType::PageOffset(o) = subtype { *o } else { 0 };

            let mut page = String::with_capacity(3);

            let host = self.target.as_service().host();
            let service = self.target.as_service().as_static_str();

            let msg_len = 21 + service.len() + 1 + user.len() + 7;
            let mut msg = String::with_capacity(msg_len);
            let _ = write!(msg, "Retrieving posts for {service}/{user} page #");

            let url_len = 8 + host.len() + 8 + service.len() + 6 + user.len() + 9;
            let mut url = String::with_capacity(url_len + 5);
            let _ = write!(url, "https://{host}/api/v1/{service}/user/{user}/posts?o=");

            loop {
                let mut retries = 0;

                let mut posts = Vec::<PagePost>::new();

                page.clear();
                let _ = write!(page, "{}", (offset + 50) / 50);

                msg.truncate(msg_len);
                let _ = write!(msg, "{page}");

                url.truncate(url_len);
                let _ = write!(url, "{offset}");

                let mut skip = false;

                loop {
                    if retries > 0 {
                        msg.truncate(msg_len + page.len());
                        let _ = write!(msg, " (Retry #{retries})");
                    }

                    msg_tx.send(msg.clone())?;

                    match post::try_fetch(&url).await {
                        Ok(p) => {
                            posts = p;
                            break;
                        }
                        Err(err) => {
                            if let PostError::MalformedPage = err {
                                skip = true;
                                break;
                            }
                            err.try_interpret(retries).await?;
                            retries += 1;
                        }
                    }
                }

                offset += 50;

                if skip {
                    continue;
                }

                if posts.is_empty() {
                    break;
                }

                for post in posts {
                    self.posts.push(Box::new(post));
                }

                if let SubType::PageOffset(_) = subtype {
                    break;
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::ref_option)]
    async fn init_posts_discord(
        &mut self,
        server: &str,
        channel: &Option<String>,
        offset: &Option<usize>
    ) -> Result<()> {
        let channels = if let Some(channel) = channel {
            vec![DiscordChannel {
                id: channel.clone(),
            }]
        } else {
            let mut url = String::with_capacity(48 + server.len());
            let _ = write!(url, "https://kemono.cr/api/v1/discord/channel/lookup/{server}");
            post::try_fetch(&url).await?
        };

        if channels.is_empty() {
            return Ok(());
        }

        let (msg_tx, msg_rx) = unbounded_channel::<String>();

        thread::spawn(move || page_progress(msg_rx));

        let mut single_page = false;

        for channel in channels {
            let mut offset = match *offset {
                Some(offset) => {
                    single_page = true;
                    offset
                }
                None => 0,
            };
            let mut page = String::with_capacity(3);

            let msg_len = 29 + server.len() + 1 + channel.id.len() + 7;
            let mut msg = String::with_capacity(msg_len);
            let _ = write!(
                msg,
                "Retrieving posts for discord/{server}/{channel} page #",
                channel = channel.id
            );

            let url_len = 41 + channel.id.len() + 3;
            let mut url = String::with_capacity(url_len + 5);
            let _ = write!(
                url,
                "https://kemono.cr/api/v1/discord/channel/{channel}?o=",
                channel = channel.id
            );

            loop {
                let mut retries = 0;

                let mut posts = Vec::<DiscordPost>::new();

                page.clear();
                let _ = write!(page, "{}", (offset + 150) / 150);

                msg.truncate(msg_len);
                let _ = write!(msg, "{page}");

                url.truncate(url_len);
                let _ = write!(url, "{offset}");

                let mut skip = false;

                loop {
                    if retries > 0 {
                        msg.truncate(msg_len + page.len());
                        let _ = write!(msg, " (Retry #{retries})");
                    }

                    msg_tx.send(msg.clone())?;

                    match post::try_fetch(&url).await {
                        Ok(p) => {
                            posts = p;
                            break;
                        }
                        Err(err) => {
                            if let PostError::MalformedPage = err {
                                skip = true;
                                break;
                            }
                            err.try_interpret(retries).await?;
                            retries += 1;
                        }
                    }
                }

                offset += 150;

                if skip {
                    break;
                }

                if posts.is_empty() {
                    break;
                }

                for post in posts {
                    self.posts.push(Box::new(post));
                }

                if single_page {
                    break;
                }
            }
        }

        Ok(())
    }

    fn init_files(&mut self) {
        self.posts.drain(..).for_each(|mut post| {
            post.files()
                .into_iter()
                .for_each(|file| {
                    self.files.insert(file);
                });
        });
    }
}
