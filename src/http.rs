use crate::cli::ARGS;
use anyhow::Result;
use reqwest::{ Client, ClientBuilder, Proxy, header::HeaderMap };
use std::{ process::exit, sync::LazyLock };

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    fn build_client() -> Result<Client> {
        let mut client = ClientBuilder::new()
            .default_headers(HeaderMap::from_iter([("accept".parse()?, "text/css".parse()?)]))
            .user_agent(format!("kumono {}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(ARGS.connect_timeout)
            .timeout(ARGS.read_timeout)
            .https_only(true);

        if let Some(proxy) = &ARGS.proxy {
            client = client.proxy(Proxy::all(proxy)?);
        }

        Ok(client.build()?)
    }

    build_client().unwrap_or_else(|err| {
        eprintln!("{err}");
        exit(1);
    })
});
