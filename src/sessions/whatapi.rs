use http::header;
use reqwest::{Client, ClientBuilder, Url};
use serde::Serialize;

pub struct WhatAPISession {
    username: String,
    password: String,
    totp: Option<String>,
    pub endpoint: Url,

    pub client: Client,
}

impl WhatAPISession {
    pub fn new(username: String, password: String, totp: Option<String>, endpoint: Url) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("User-Agent", "orpheusmorebetter".parse().unwrap());

        let client = ClientBuilder::new()
            .cookie_store(true)
            .default_headers(headers)
            .build()
            .expect("API client construction failed");

        let out = WhatAPISession {
            username: username,
            password: password,
            totp: totp,
            endpoint: endpoint,
            client: client,
        };

        return out;
    }

    pub async fn log_in(&self) -> Result<reqwest::Response, reqwest::Error> {
        // populate session cookies
        let body = LoginBody {
            username: &self.username,
            password: &self.password,
            mfa: self.totp.as_deref(),
            login: "Log in",
        };

        let res = self
            .client
            .post(self.endpoint.clone())
            .json(&body)
            .send()
            .await?
            .error_for_status();

        return res;
    }
}

#[derive(Serialize)]
struct LoginBody<'a> {
    username: &'a str,
    password: &'a str,
    mfa: Option<&'a str>,
    login: &'static str, // HAS TO be "Log in"
}
