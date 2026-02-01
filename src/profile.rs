use crate::{
    api::{ self, DiscordChannel, DiscordPost, PagePost, Post, SinglePost },
    file::PostFile,
    http::CLIENT,
    pretty::{ self, n_fmt },
    target::{ SubType, Target },
};
use anyhow::Result;
use indicatif::{ ProgressBar, ProgressStyle };
use std::{ collections::HashSet, fmt::{ self, Display, Formatter, Write }, sync::Arc, thread };
use tokio::{ sync::mpsc::{ UnboundedReceiver, unbounded_channel }, time::{ Duration, sleep } };

pub struct Profile {
    target_id: usize,
    target: Arc<Target>,
    post_count: usize,
    posts: Vec<Box<dyn Post>>,
    pub files: HashSet<PostFile>,
}

impl Display for Profile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if
            let
            | Target::Creator { subtype: SubType::Post(_), .. }
            | Target::Discord { channel: None, .. } = *self.target
        {
            write!(
                f,
                "#{number}: {target} has {posts}",
                number = n_fmt(self.target_id as u64),
                target = self.target,
                posts = pretty::with_word(self.files.len() as u64, "file")
            )?;
        } else {
            write!(
                f,
                "#{number}: {target} has {posts}",
                number = n_fmt(self.target_id as u64),
                target = self.target,
                posts = pretty::with_word(self.post_count as u64, "post")
            )?;

            if self.post_count > 0 {
                match self.files.len() {
                    0 => write!(f, ", but no files")?,
                    1 => write!(f, ", containing 1 file")?,
                    n => write!(f, ", containing {} files", n_fmt(n as u64))?,
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
        let mut profile = Self {
            target_id,
            target: target.clone(),
            post_count: 0,
            posts: Vec::with_capacity(250),
            files: HashSet::new(),
        };

        match &*target {
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
            let post: SinglePost = CLIENT.get({
                let (host, service) = (
                    self.target.as_service().host(),
                    self.target.as_service().as_static_str(),
                );

                let mut url = String::with_capacity(
                    8 + host.len() + 8 + service.len() + 6 + user.len() + 6 + post.len()
                );
                write!(url, "https://{host}/api/v1/{service}/user/{user}/post/{post}")?;

                url
            })
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
                    let target = self.target.to_string();
                    let page = ((offset + 50) / 50).to_string();

                    let mut msg = String::with_capacity(
                        21 + target.len() + 7 + page.len() + 9 + 1 + 1
                    );

                    write!(msg, "Retrieving posts for {target} page #{page}")?;

                    if retries > 0 {
                        write!(msg, " (Retry #{retries})")?;
                    }

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
            api::try_discord_server(server).await?
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

                let page = ((offset + 150) / 150).to_string();

                let mut msg = String::with_capacity(
                    29 + server.len() + 1 + channel.id.len() + 7 + page.len() + 9 + 1 + 1
                );

                loop {
                    write!(
                        msg,
                        "Retrieving posts for discord/{server}/{} page #{page}",
                        channel.id
                    )?;

                    if retries > 0 {
                        write!(msg, " (Retry #{retries})")?;
                    }

                    msg_tx.send(msg.clone())?;

                    match api::try_discord_page(&channel.id, offset).await {
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
