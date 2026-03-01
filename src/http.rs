use crate::cli::ARGUMENTS;
use anyhow::{ Result, anyhow };
use reqwest::{ Client, ClientBuilder, Proxy, header::{ HeaderMap, HeaderValue }, redirect::Policy };
use serde::Deserialize;
use serde_json::json;
use std::{ fmt::Write, process::exit, sync::LazyLock };

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
        Some(("coomer.st", user, pass))
    } else {
        None
    };

    let kemono_auth = if
        let Some(user) = ARGUMENTS.kemono_user.as_ref() &&
        let Some(pass) = ARGUMENTS.kemono_pass.as_ref()
    {
        Some(("kemono.cr", user, pass))
    } else {
        None
    };

    // add validation: non-zero length, charset?

    for (host, user, pass) in [coomer_auth, kemono_auth].into_iter().flatten() {
        let mut url = String::with_capacity(8 + host.len() + 28);
        let _ = write!(url, "https://{host}/api/v1/authentication/login");

        let json = json!({"username": user, "password": pass});

        let mut tries = 0;

        loop {
            tries += 1;

            match CLIENT.post(&url).json(&json).send().await {
                Ok(res) =>
                    match res.json().await? {
                        LoginResponse::Success { .. } => {
                            break;
                        }
                        LoginResponse::Error { error } => {
                            return Err(anyhow!(error));
                        }
                    }

                Err(err) => {
                    if tries == ARGUMENTS.max_tries {
                        return Err(anyhow!(err));
                    }
                }
            }
        }
    }

    Ok(())
}
