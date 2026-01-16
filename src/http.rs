use crate::cli::ARGUMENTS;
use anyhow::Result;
use reqwest::{
    Client,
    ClientBuilder,
    Proxy,
    header::{ HeaderMap, HeaderName, HeaderValue },
    redirect::Policy,
};
use std::{ process::exit, sync::LazyLock };

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    fn build_client() -> Result<Client> {
        let mut client = ClientBuilder::new()
            .default_headers(
                HeaderMap::from_iter([
                    (HeaderName::from_static("accept"), HeaderValue::from_static("text/css")),
                ])
            )
            .user_agent(format!("kumono {}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(ARGUMENTS.connect_timeout)
            .timeout(ARGUMENTS.read_timeout)
            .redirect(Policy::limited(1))
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
