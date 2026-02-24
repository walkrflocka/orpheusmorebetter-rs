use chrono::{DateTime, TimeDelta, Utc};
use reqwest::Url;

use crate::sessions::whatapi::WhatAPISession;

#[derive(Debug)]
pub struct WhatAPI {
    // user input fields

    // internal fields
    last_request: DateTime<Utc>,
    pub api_session: WhatAPISession,
}

impl WhatAPI {
    const MIN_SEC_BETWEEN_REQUEST: TimeDelta =
        TimeDelta::try_seconds(5).expect("Failed to construct TimeDelta in WhatAPI");

    pub async fn new(
        username: String,
        password: String,
        endpoint: Option<String>,
        totp: Option<String>,
    ) -> anyhow::Result<Self> {
        // trailing slash IMPORTANT
        let resolved_endpoint: Url =
            parse_url_with_fallback(endpoint, String::from("https://orpheus.network/"));

        let api_session =
            WhatAPISession::new(username, password, totp, resolved_endpoint).await?;

        Ok(WhatAPI {
            last_request: Utc::now(),
            api_session,
        })
    }
}

fn parse_url_with_fallback(url_str: Option<String>, fallback: String) -> Url {
    let mut s = url_str.unwrap_or(fallback);
    if s.chars().last() != Some('/') {
        s.push_str("/");
    }

    return s.parse::<Url>().expect("API endpoint parsing failed.");
}
