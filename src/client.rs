use crate::config::CONFIG;
use log::error;
use reqwest::{Client, ClientBuilder, Proxy};
use std::process;

lazy_static::lazy_static! {
    pub static ref CLIENT: Client = {
        let mut client = ClientBuilder::new().connect_timeout(CONFIG.connect_timeout());

        if let Some(proxy) = CONFIG.proxy() {
            client = client.proxy(Proxy::all(proxy)
                .unwrap_or_else(|err| {
                    error!("{err}");
                    process::exit(1);
                }
            ));
        }

        client.build().unwrap()
    };
}
