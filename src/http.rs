use crate::cli::ARGUMENTS;
use anyhow::Result;
use reqwest::{ Client, ClientBuilder, Proxy, header::{ HeaderMap, HeaderValue }, redirect::Policy };
use std::{ process::exit, sync::LazyLock };

static VERSION: &str = concat!("kumono ", env!("CARGO_PKG_VERSION"));

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let build_client = || -> Result<Client> {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/css"));

        let mut client = ClientBuilder::new()
            .default_headers(headers)
            .user_agent(VERSION)
            .connect_timeout(ARGUMENTS.connect_timeout)
            .timeout(ARGUMENTS.read_timeout)
            .redirect(Policy::limited(1))
            .https_only(true)
            .http2_prior_knowledge();

        if let Some(proxy) = &ARGUMENTS.proxy {
            client = client.proxy(Proxy::all(proxy)?);
        }

        Ok(client.build()?)
    };

    build_client().unwrap_or_else(|err| {
        eprintln!("{err}");
        exit(2);
    })
});
