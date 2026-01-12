use crate::{
    api::{ self, DiscordChannel, DiscordPost, PagePost, Post, SinglePost },
    file::PostFile,
    http::CLIENT,
    pretty::{ self, n_fmt },
    target::{ SubType, Target },
};
use anyhow::Result;
use indicatif::{ ProgressBar, ProgressStyle };
use std::{ collections::HashSet, fmt::{ self, Display, Formatter }, thread };
use tokio::{ sync::mpsc::{ UnboundedReceiver, unbounded_channel }, time::{ Duration, sleep } };

pub struct Profile {
    target_id: usize,
    pub target: Target,
    post_count: usize,
    posts: Vec<Box<dyn Post>>,
    pub files: HashSet<PostFile>,
}

impl Display for Profile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if
            let
            | Target::Creator { subtype: SubType::Post(_), .. }
            | Target::Discord { channel: None, .. } = &self.target
        {
            write!(
                f,
                "#{number}: {target} has {posts}",
                number = n_fmt(self.target_id as u64),
                target = self.target,
                posts = pretty::with_noun(self.files.len() as u64, "file")
            )
        } else {
            let files = if self.post_count == 0 {
                ""
            } else {
                match self.files.len() {
                    0 => ", but no files",
                    1 => ", containing 1 file",
                    n => &format!(", containing {number} files", number = n_fmt(n as u64)),
                }
            };

            write!(
                f,
                "#{number}: {target} has {posts}{files}",
                number = n_fmt(self.target_id as u64),
                target = self.target,
                posts = pretty::with_noun(self.post_count as u64, "post")
            )
        }
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
    pub async fn try_new(target: &Target, target_id: usize) -> Result<Self> {
        let mut profile = Self {
            target_id,
            target: target.clone(),
            post_count: 0,
            posts: Vec::with_capacity(250),
            files: HashSet::new(),
        };

        match target {
            Target::Creator { user, subtype, .. } => {
                profile.init_posts_standard(user, subtype).await?;
            }
            Target::Discord { server, channel, .. } => {
                profile.init_posts_discord(server, channel).await?;
            }
        }

        // wait for progress bar to finish
        sleep(Duration::from_millis(1)).await;

        profile.init_files();

        eprintln!("{profile}");

        // discard all posts
        profile.posts.clear();

        Ok(profile)
    }

    async fn init_posts_standard(&mut self, user: &str, subtype: &SubType) -> Result<()> {
        if let SubType::Post(post) = subtype {
            let post: SinglePost = CLIENT.get(
                format!(
                    "https://{site}/api/v1/{service}/user/{user}/post/{post}",
                    site = self.target.as_service().site(),
                    service = self.target.as_service()
                )
            )
                .send().await?
                .json().await?;

            self.posts.push(Box::new(post));
        } else {
            let (msg_tx, msg_rx) = unbounded_channel::<String>();

            thread::spawn(move || page_progress(msg_rx));

            let mut offset = if let SubType::PageOffset(o) = subtype { *o } else { 0 };

            loop {
                let mut retries = 0;

                let posts: Vec<PagePost>;

                loop {
                    let msg = format!(
                        "Retrieving posts for {target} page #{page}{try}",
                        target = self.target,
                        page = (offset + 50) / 50,
                        r#try = if retries > 0 {
                            format!(" (Retry #{retries})")
                        } else {
                            String::new()
                        }
                    );

                    msg_tx.send(msg)?;

                    match api::try_fetch_page(&self.target, user, offset).await {
                        Ok(p) => {
                            posts = p;
                            break;
                        }
                        Err(err) => {
                            err.try_interpret(retries).await?;
                            retries += 1;
                        }
                    }
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

                offset += 50;
            }
        }

        Ok(())
    }

    #[allow(clippy::ref_option)]
    async fn init_posts_discord(&mut self, server: &str, channel: &Option<String>) -> Result<()> {
        let channels = if let Some(channel) = channel {
            vec![DiscordChannel {
                id: channel.clone(),
            }]
        } else {
            api::discord_server(server).await?
        };

        if channels.is_empty() {
            return Ok(());
        }

        let (msg_tx, msg_rx) = unbounded_channel::<String>();

        thread::spawn(move || page_progress(msg_rx));

        for channel in channels {
            let mut offset = 0;

            loop {
                let mut retries = 0;

                let posts: Vec<DiscordPost>;

                loop {
                    let msg = format!(
                        "Retrieving posts for discord/{server}/{channel} page #{page}{retry}",
                        channel = channel.id,
                        page = (offset + 150) / 150,
                        retry = if retries > 0 {
                            format!(" (Retry #{retries})")
                        } else {
                            String::new()
                        }
                    );

                    msg_tx.send(msg)?;

                    match api::discord_page(&channel.id, offset).await {
                        Ok(p) => {
                            posts = p;
                            break;
                        }
                        Err(err) => {
                            err.try_interpret(retries).await?;
                            retries += 1;
                        }
                    }
                }

                if posts.is_empty() {
                    break;
                }

                for post in posts {
                    self.posts.push(Box::new(post));
                }

                offset += 150;
            }
        }

        Ok(())
    }

    fn init_files(&mut self) {
        self.post_count = self.posts.len();

        self.posts.drain(..).for_each(|mut post| {
            post.files()
                .into_iter()
                .for_each(|file| {
                    self.files.insert(file);
                });
        });
    }
}
