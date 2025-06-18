use crate::cli::ARGS;
use reqwest::{ Client, ClientBuilder, Proxy };
use std::{ process, sync::LazyLock };

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let mut client = ClientBuilder::new().connect_timeout(ARGS.connect_timeout);

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
