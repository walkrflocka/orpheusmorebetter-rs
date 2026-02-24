use chrono::{DateTime, TimeDelta, Utc};
use futures::executor::block_on;
use reqwest::{ClientBuilder, Url};
use serde::{Deserialize, Serialize};

use crate::sessions::ajax::AjaxSession;
use crate::sessions::whatapi::WhatAPISession;

struct WhatAPI {
    // user input fields

    // internal fields
    last_request: DateTime<Utc>,
    api_session: WhatAPISession,
    ajax_session: AjaxSession,

    user_id: Option<String>,
    authkey: Option<String>,
    passkey: Option<String>,
}

impl WhatAPI {
    const MIN_SEC_BETWEEN_REQUEST: TimeDelta =
        TimeDelta::try_seconds(5).expect("Failed to construct TimeDelta in WhatAPI");

    pub fn new(username: String, password: String, endpoint: Option<String>, totp: Option<String>) {
        // trailing slash IMPORTANT
        let resolved_endpoint: Url =
            parse_url_with_fallback(endpoint, String::from("https://orpheus.network/"));

        // when you auth to orpheus you get a cookie back that you need to use
        // to auth for future requests - unfortunately it don't work with basic
        // HTTP auth :(

        let client = WhatAPISession::new(username, password, totp, resolved_endpoint);

        let mut out: WhatAPI = WhatAPI {
            last_request: Utc::now(),
            api_session: client,
        };

        // TODO: improve handling around bad results
        let login_fut = out.api_session.log_in();
        block_on(login_fut).expect("Login to Orpheus failed.");

        let keys_fut = out.get_keys();
        let keys =
            block_on(keys_fut).expect("Failed to parse auth/passkeys from Orpheus response.");

        out.authkey = keys.authkey;
        out.passkey = keys.authkey;
        out.user_id = keys.id;

        return out;
    }

    async fn get_keys(&self) -> Result<IndexBody, anyhow::Error> {
        let url = self
            .endpoint
            .join("ajax.php")
            .expect("Unable to construct Ajax endpoint");

        let res = self
            .api_session
            .get(url)
            .query(&[("action", "index")])
            .send()
            .await?;

        let status = res.status();
        if status.is_client_error() || status.is_server_error() {
            anyhow::bail!(
                "Orpheus ajax.php?action=index request failed with code {}",
                status.as_str()
            )
        }

        let keys = res.json::<IndexBody>().await?;

        return Ok(keys);
    }
}

fn parse_url_with_fallback(url_str: Option<String>, fallback: String) -> Url {
    let mut s = url_str.unwrap_or(fallback);
    if s.chars().last() != Some('/') {
        s.push_str("/");
    }

    return s.parse::<Url>().expect("API endpoint parsing failed.");
}

#[derive(Deserialize)]
struct IndexBody {
    id: String,
    authkey: String,
    passkey: String,
}
