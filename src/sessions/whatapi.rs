use futures::executor::block_on;
use http::{Method, header, method};
use reqwest::{Client, ClientBuilder, RequestBuilder, Url};
use serde::{Deserialize, Serialize};

pub struct WhatAPISession {
    username: String,
    password: String,
    totp: Option<String>,
    pub endpoint: Url,

    pub client: Client,

    pub user_id: String,
    authkey: String,
    passkey: String,
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

        let login_fut =
            WhatAPISession::log_in(&client, &username, &password, &endpoint, totp.as_deref());
        block_on(login_fut).expect("Login to Orpheus failed.");
        let keys_fut = WhatAPISession::get_keys(&client, &endpoint);
        let keys =
            block_on(keys_fut).expect("Failed to parse auth/passkeys from Orpheus response.");

        let out = WhatAPISession {
            username: username,
            password: password,
            totp: totp,
            endpoint: endpoint,
            client: client,
            user_id: keys.id,
            passkey: keys.passkey,
            authkey: keys.authkey,
        };

        return out;
    }

    pub async fn log_in(
        client: &Client,
        username: &str,
        password: &str,
        endpoint: &Url,
        mfa: Option<&str>,
    ) -> anyhow::Result<()> {
        // populate session cookies
        let body = LoginBody {
            username: username,
            password: password,
            mfa: mfa,
            login: "Log in",
        };

        let res = client.post(endpoint.clone()).json(&body).send().await?;

        let status = res.status();
        if status.is_client_error() || status.is_server_error() {
            anyhow::bail!(
                "Orpheus ajax.php?action=index request failed with code {}",
                status.as_str()
            )
        }

        return Ok(());
    }

    async fn get_keys(client: &Client, endpoint: &Url) -> Result<IndexBody, anyhow::Error> {
        let url = endpoint
            .join("ajax.php")
            .expect("Unable to construct Ajax endpoint");

        let res = client.get(url).query(&[("action", "index")]).send().await?;

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

    pub fn new_http_request(&self, method: Method) -> RequestBuilder {
        return self.client.request(method, self.endpoint.clone());
    }

    pub fn new_ajax_request(&self, method: Method, action: String) -> RequestBuilder {
        return self
            .new_http_request(method)
            .query(&[("auth", &self.authkey), ("action", &action)]);
    }
}

#[derive(Serialize)]
struct LoginBody<'a> {
    username: &'a str,
    password: &'a str,
    mfa: Option<&'a str>,
    login: &'static str, // HAS TO be "Log in"
}

#[derive(Deserialize)]
struct IndexBody {
    id: String,
    authkey: String,
    passkey: String,
}
