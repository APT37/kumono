use std::process;

use clap::ValueEnum;
use serde::Deserialize;
use strum_macros::Display;

#[allow(non_camel_case_types)]
#[derive(Deserialize, Display, Clone, Copy, ValueEnum)]
pub enum Service {
    afdian,
    boosty,
    candfans,
    discord,
    dlsite,
    fanbox,
    fansly,
    fantia,
    gumroad,
    onlyfans,
    patreon,
    subscribestar,
}

impl Service {
    #[must_use]
    pub fn site(self) -> &'static str {
        use Service::{ candfans, discord, fansly, onlyfans };

        match self {
            candfans | fansly | onlyfans => "coomer",
            discord => {
                eprintln!("Discord support is not implemented.");
                process::exit(1);
            }
            _ => "kemono",
        }
    }
}
