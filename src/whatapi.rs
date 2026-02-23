use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use futures::executor::block_on;
use reqwest::Method;
use reqwest::{Client, ClientBuilder, Url};
use serde::Serialize;

struct WhatAPI<'a> {
    // user input fields
    username: &'a str,
    password: &'a str,
    totp: &'a str,
    endpoint: Url,

    // internal fields
    last_request: DateTime<Utc>,
    api_client: Client,

    user_id: Option<String>,
    authkey: Option<String>,
    passkey: Option<String>,
}

impl WhatAPI<'_> {
    const MIN_SEC_BETWEEN_REQUEST: TimeDelta =
        TimeDelta::try_seconds(5).expect("Failed to construct TimeDelta in WhatAPI");

    pub fn new(username: &str, password: &str, endpoint: Option<String>, totp: Option<&str>) {
        // trailing slash IMPORTANT
        let resolved_endpoint: Url =
            parse_url_with_fallback(endpoint, String::from("https://orpheus.network/"));

        // when you auth to orpheus you get a cookie back that you need to use
        // to auth for future requests - unfortunately it don't work with basic
        // HTTP auth :(
        let client = ClientBuilder::new()
            .cookie_store(true)
            .build()
            .expect("API client construction failed");

        let out: WhatAPI<'_> = WhatAPI {
            username: username,
            password: password,
            totp: totp.unwrap(),
            last_request: Utc::now(),
            api_client: client,
            endpoint: resolved_endpoint,
        };

        // TODO: improve handling around bad results
        let login_fut = out.log_in();
        block_on(login_fut).expect("Login to Orpheus failed.");
    }

    async fn log_in(&self) -> Result<reqwest::Response, reqwest::Error> {
        let body = LoginBody {
            username: self.username,
            password: self.password,
            mfa: self.totp,
            login: "Log in",
        };

        let res = self
            .api_client
            .post(self.endpoint.clone())
            .json(&body)
            .send()
            .await?
            .error_for_status();

        return res;
    }

    async fn request_ajax(
        &self,
        action: &str,
        data: HashMap<&str, &str>,
        method: Method,
        params: HashMap<&str, &str>,
    ) {
    }

    async fn get_keys(&self) {}
}

fn parse_url_with_fallback(url_str: Option<String>, fallback: String) -> Url {
    let mut s = url_str.unwrap_or(fallback);
    if s.chars().last() != Some('/') {
        s.push_str("/");
    }

    return s.parse::<Url>().expect("API endpoint parsing failed.");
}

#[derive(Serialize)]
struct LoginBody<'a> {
    username: &'a str,
    password: &'a str,
    mfa: &'a str,
    login: &'static str, // HAS TO be "Log in"
}
