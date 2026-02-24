use http::Method;
use reqwest::{
    Client, ClientBuilder, RequestBuilder, Url,
    header::{HeaderMap, SET_COOKIE},
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct WhatAPISession {
    username: String,
    password: String,
    totp: Option<String>,
    pub endpoint: Url,

    pub client: Client,

    pub user_id: u32,
    authkey: String,
    passkey: String,
}

impl WhatAPISession {
    pub async fn new(
        username: String,
        password: String,
        totp: Option<String>,
        endpoint: Url,
    ) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "orpheusmorebetter".parse().unwrap());

        let client = ClientBuilder::new()
            .cookie_store(true)
            .default_headers(headers)
            .build()
            .expect("API client construction failed");

        WhatAPISession::log_in(&client, &username, &password, &endpoint, totp.as_deref()).await?;

        let keys = WhatAPISession::get_keys(&client, &endpoint).await?;
        println!("Logged into Orpheus.");

        Ok(WhatAPISession {
            username,
            password,
            totp,
            endpoint,
            client,
            user_id: keys.id,
            passkey: keys.passkey,
            authkey: keys.authkey,
        })
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

        // when you auth to orpheus you get a cookie back that you need to use
        // to auth for future requests - unfortunately it don't work with basic
        // HTTP auth :(

        let res = client
            .post(
                endpoint
                    .join("login.php")
                    .expect("Failed to construct login endpoint"),
            )
            .form(&body)
            .send()
            .await?;

        let status = res.status();
        if status.is_client_error() || status.is_server_error() {
            anyhow::bail!(
                "Orpheus ajax.php?action=index request failed with code {}",
                status.as_str()
            )
        }

        return Ok(());
    }

    async fn get_keys(client: &Client, endpoint: &Url) -> Result<IndexKeys, anyhow::Error> {
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

        let res_body = res.json::<IndexBody>().await;

        match res_body {
            Ok(r) => return Ok(r.response),
            Err(_) => panic!(
                "Could not retrieve user id/keys from Orpheus. Login most likely failed. Check your user/pass?"
            ),
        }
    }

    pub fn new_http_request(&self, method: Method) -> RequestBuilder {
        return self.client.request(method, self.endpoint.clone());
    }

    pub fn new_ajax_request(&self, method: Method, action: String) -> RequestBuilder {
        return self
            .new_http_request(method)
            .query(&[("authkey", &self.authkey), ("action", &action)]);
    }
}

#[derive(Serialize)]
struct LoginBody<'a> {
    username: &'a str,
    password: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    mfa: Option<&'a str>,
    login: &'a str,
}

#[derive(Deserialize)]
struct IndexBody {
    status: String,
    response: IndexKeys,
}

#[derive(Deserialize)]
struct IndexKeys {
    id: u32,
    authkey: String,
    passkey: String,
}
