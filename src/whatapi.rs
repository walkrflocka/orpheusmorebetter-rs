use chrono::{DateTime, TimeDelta, Utc};
use reqwest::Url;

use crate::sessions::whatapi::WhatAPISession;

pub struct WhatAPI {
    // user input fields

    // internal fields
    last_request: DateTime<Utc>,
    api_session: WhatAPISession,
}

impl WhatAPI {
    const MIN_SEC_BETWEEN_REQUEST: TimeDelta =
        TimeDelta::try_seconds(5).expect("Failed to construct TimeDelta in WhatAPI");

    pub fn new(
        username: String,
        password: String,
        endpoint: Option<String>,
        totp: Option<String>,
    ) -> Self {
        // trailing slash IMPORTANT
        let resolved_endpoint: Url =
            parse_url_with_fallback(endpoint, String::from("https://orpheus.network/"));

        let client = WhatAPISession::new(username, password, totp, resolved_endpoint);

        let out: WhatAPI = WhatAPI {
            last_request: Utc::now(),
            api_session: client,
        };

        return out;
    }
}

fn parse_url_with_fallback(url_str: Option<String>, fallback: String) -> Url {
    let mut s = url_str.unwrap_or(fallback);
    if s.chars().last() != Some('/') {
        s.push_str("/");
    }

    return s.parse::<Url>().expect("API endpoint parsing failed.");
}
