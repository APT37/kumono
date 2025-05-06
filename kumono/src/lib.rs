use clap::ValueEnum;
use serde::Deserialize;
use strum_macros::Display;

#[allow(non_camel_case_types)]
#[derive(Display, Deserialize, Clone, Copy, ValueEnum)]
pub enum Service {
    boosty,
    candfans,
    // discord,
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
        use Service::{ candfans, fansly, onlyfans };

        match self {
            candfans | fansly | onlyfans => "coomer",
            _ => "kemono",
        }
    }
}
