use crate::config::CONFIG;
use log::error;
use reqwest::{Client, ClientBuilder, Proxy};
use std::{process, sync::LazyLock};

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let mut client = ClientBuilder::new().connect_timeout(CONFIG.connect_timeout());

    if let Some(proxy) = CONFIG.proxy() {
        client = client.proxy(Proxy::all(proxy).unwrap_or_else(|err| {
            error!("{err}");
            process::exit(1);
        }));
    }

    client.build().unwrap()
});
