use anyhow::Result;
use futures::future::join_all;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use std::{env, path::PathBuf, process, str::FromStr};
use tokio::{fs, io::AsyncWriteExt, task};

lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::new();
}

struct Target {
    creator: String,
    url: String,
}

impl Target {
    fn from_args() -> Self {
        let args: Vec<_> = env::args().filter(|arg| !arg.is_empty()).collect();

        if args.len() != 3 {
            eprintln!("Usage: coomer-rip <service> <creator>");
            eprintln!();
            eprintln!("Services: onlyfans, fansly, fanbox, patreon, ...");
            eprintln!();
            eprintln!("Example: coomer-rip onlyfans belledelphine");
            process::exit(1);
        }

        Self {
            creator: args[2].clone(),
            url: format!("https://coomer.su/api/v1/{}/user/{}", args[1], args[2]),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let target = Target::from_args();

    fs::create_dir_all(&target.creator).await?;

    let mut failed = 0;

    for post in get_posts(&target).await? {
        let mut tasks = vec![];

        for file in post.files() {
            let creator = target.creator.clone();

            tasks.push(task::spawn(async move {
                if let Err(err) = file.download(creator).await {
                    eprintln!("Error Downloading {}: {err}", file.name.unwrap());
                }
            }));
        }

        for task in join_all(tasks).await {
            if task.is_err() {
                failed += 1;
            }
        }
    }

    if failed > 0 {
        eprintln!("\nFailed to download:");
    }

    Ok(())
}

async fn get_posts(target: &Target) -> Result<Vec<Post>> {
    let mut all_posts = vec![];

    let mut offset = 0;

    loop {
        let mut posts: Vec<Post> = CLIENT
            .get(format!("{}?o={offset}", target.url))
            .send()
            .await?
            .json()
            .await?;

        if posts.is_empty() {
            break;
        }

        all_posts.append(&mut posts);

        offset += 50;
    }

    Ok(all_posts)
}

#[derive(Deserialize)]
struct Post {
    // id: String,        // "1000537173"
    // user: String,      // "paigetheuwulord"
    // service: String,  // "onlyfans"
    // title: String,     // "What an ass"
    // content: String,   // "What an ass"
    // shared_file: bool, // false
    // added: String,     // "2024-04-04T15:52:37.557866"
    // published: String, // "2024-03-31T00:53:25"
    // edited: ???, // null
    file: Option<PostFile>,
    attachments: Vec<PostFile>,
    // poll: ???, // null
    // captions: ???, // null
    // tags: ???, // null
}

impl Post {
    fn files(&self) -> Vec<PostFile> {
        let mut files = vec![];

        if let Some(file) = &self.file {
            files.push(file.clone());
        }

        for file in &self.attachments {
            files.push(file.clone());
        }

        files
    }
}

#[derive(Deserialize, Clone)]
struct PostFile {
    name: Option<String>, // "1242x2208_882b040faaac0e38fba20f4caadb2e59.jpg",
    path: Option<String>, // "/6e/6c/6e6cf84df44c1d091a2e36b6df77b098107c18831833de1e2e9c8207206f150b.jpg"
}

impl PostFile {
    async fn download(&self, creator: impl AsRef<str>) -> Result<()> {
        if let (Some(name), Some(path)) = (&self.name, &self.path) {
            let fs_path = PathBuf::from_str(&format!("{}/{name}", creator.as_ref()))?;

            if fs::try_exists(&fs_path).await? {
                println!("Skipping {}", fs_path.to_string_lossy());
            } else {
                println!("Downloading {}", fs_path.to_string_lossy());

                let mut stream = CLIENT
                    .get(format!("https://coomer.su/data{path}"))
                    .send()
                    .await?
                    .bytes_stream();

                let mut file = fs::File::create_new(&fs_path).await?;

                while let Some(Ok(chunk)) = stream.next().await {
                    file.write_all(&chunk).await?;
                }

                file.flush().await?;
            }
        }

        Ok(())
    }
}
