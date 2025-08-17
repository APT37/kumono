use crate::cli::ARGS;
use reqwest::{ Client, ClientBuilder, Proxy, header::{ HeaderMap, HeaderValue } };
use std::{ process, sync::LazyLock };

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", HeaderValue::from_static("Text/CSS"));

    let mut client = ClientBuilder::new()
        .default_headers(headers)
        .connect_timeout(ARGS.connect_timeout)
        .timeout(ARGS.read_timeout)
        .https_only(true);

    if let Some(proxy) = &ARGS.proxy {
        client = client.proxy(
            Proxy::all(proxy).unwrap_or_else(|err| {
                eprintln!("{err}");
                process::exit(1);
            })
        );
    }

    client.build().unwrap()
});
