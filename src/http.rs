use crate::cli::ARGS;
use reqwest::{ Client, ClientBuilder, Proxy, header::{ HeaderMap, HeaderValue }, cookie::Jar };
use std::{ process, sync::{ Arc, LazyLock } };

static COOKIE_JAR: LazyLock<Arc<Jar>> = LazyLock::new(|| Arc::new(Jar::default()));

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", HeaderValue::from_static("text/css"));

    let mut client = ClientBuilder::new()
        .default_headers(headers)
        .cookie_provider(Arc::clone(&COOKIE_JAR))
        .connect_timeout(ARGS.connect_timeout)
        .timeout(ARGS.read_timeout)
        .https_only(true)
;

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

pub static AUTH_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let mut client = ClientBuilder::new()
        .default_headers(headers)
        .cookie_provider(Arc::clone(&COOKIE_JAR))
        .connect_timeout(ARGS.connect_timeout)
        .timeout(ARGS.read_timeout)
        .https_only(true)
;

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
