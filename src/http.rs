use crate::cli::ARGUMENTS;
use anyhow::Result;
use reqwest::{ Client, ClientBuilder, Proxy, header::HeaderMap };
use std::{ process::exit, sync::LazyLock };

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    fn build_client() -> Result<Client> {
        let mut client = ClientBuilder::new()
            .default_headers(HeaderMap::from_iter([("accept".parse()?, "text/css".parse()?)]))
            .user_agent(format!("kumono {version}", version = env!("CARGO_PKG_VERSION")))
            .connect_timeout(ARGUMENTS.connect_timeout)
            .timeout(ARGUMENTS.read_timeout)
            .https_only(true);

        if let Some(proxy) = &ARGUMENTS.proxy {
            client = client.proxy(Proxy::all(proxy)?);
        }

        Ok(client.build()?)
    }

    build_client().unwrap_or_else(|err| {
        eprintln!("{err}");
        exit(1);
    })
});
