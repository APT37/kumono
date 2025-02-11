use crate::config;
use reqwest::{Client, ClientBuilder, Proxy};

lazy_static::lazy_static! {
    pub static ref CLIENT: Client = ClientBuilder::new()
        .connect_timeout(config::CONNECT_TIMEOUT)
        .proxy(Proxy::all(config::PROXY).unwrap())
        .build()
        .unwrap();
}
