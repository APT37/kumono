use crate::cli::ARGUMENTS;
use anyhow::{ Result, anyhow };
use reqwest::{ Client, ClientBuilder, Proxy, header::{ HeaderMap, HeaderValue }, redirect::Policy };
use serde::Deserialize;
use serde_json::json;
use std::{ process::exit, sync::LazyLock };

static VERSION: &str = concat!("kumono ", env!("CARGO_PKG_VERSION"));

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let build_client = || -> Result<Client> {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/css"));

        let mut client = ClientBuilder::new()
            .default_headers(headers)
            .user_agent(VERSION)
            .cookie_store(true)
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

#[allow(unused)]
#[derive(Deserialize)]
#[serde(untagged)]
enum LoginResponse {
    Success {
        id: usize,
        username: String,
        created_at: String,
        role: String,
        // is_duplicate: bool,
    },
    Error {
        error: String,
    },
}

// login must only be attempted once. trying to log in with valid login cookies present
// results in an HTTP 409 status code and an API error: {"error":"Already logged in"}
pub async fn try_login() -> Result<()> {
    let coomer_auth = if
        let Some(user) = ARGUMENTS.coomer_user.as_ref() &&
        let Some(pass) = ARGUMENTS.coomer_pass.as_ref()
    {
        Some((user, pass, "https://coomer.st/api/v1/authentication/login"))
    } else {
        None
    };

    let kemono_auth = if
        let Some(user) = ARGUMENTS.kemono_user.as_ref() &&
        let Some(pass) = ARGUMENTS.kemono_pass.as_ref()
    {
        Some((user, pass, "https://kemono.cr/api/v1/authentication/login"))
    } else {
        None
    };

    // add validation: non-zero length, charset?

    for (user, pass, url) in [coomer_auth, kemono_auth].into_iter().flatten() {
        match
            CLIENT.post(url)
                .json(&json!({"username": user, "password": pass}))
                .send().await?
                .json().await?
        {
            LoginResponse::Success { .. } => {}
            LoginResponse::Error { error } => {
                return Err(anyhow!(error));
            }
        }
    }

    Ok(())
}
