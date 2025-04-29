use clap::{ arg, Parser, ValueEnum };
use serde::Deserialize;
use std::sync::LazyLock;
use strum_macros::Display;

pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

#[derive(Deserialize, Parser)]
pub struct Args {
    #[arg(short, long)]
    pub service: Service,

    #[arg(short = 'i', long = "id")]
    pub creator: String,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Display, Clone, Copy, ValueEnum)]
pub enum Service {
    boosty,
    candfans,
    discord,
    dlsite,
    fanBox,
    fansly,
    fantia,
    gumroad,
    onlyfans,
    patreon,
    subscribestar,
}

impl Service {
    pub fn site(self) -> &'static str {
        use Service::{ candfans, fansly, onlyfans };

        match self {
            candfans | fansly | onlyfans => "coomer",
            _ => "kemono",
        }
    }
}
