use reqwest::{Body, Client, Url};

pub struct AjaxSession {
    authkey: String,
    url: Url,
    client: &Client,
}

impl AjaxSession {
    // TODO: implement separation of client and cookie store, maybe a struct?
    // I need to be able to set a contract that this session only takes
    // a session with cookies attached
    pub fn new(client: &Client, authkey: String, base_url: Url) -> Self {
        return AjaxSession {
            authkey,
            url: base_url
                .join("ajax.php")
                .expect("Failed to construct Ajax endpoint"),
            client: client,
        };
    }

    fn add_query_params(
        &self,
        builder: reqwest::RequestBuilder,
        action: &str,
    ) -> reqwest::RequestBuilder {
        builder.query(&[
            ("authkey", self.authkey.clone()),
            ("action", action.to_string()),
        ])
    }

    pub async fn get(&self, action: &str) -> Result<reqwest::Response, reqwest::Error> {
        let builder = self.client.get(self.url.clone());
        let res = self.add_query_params(builder, action).send().await;

        return res;
    }

    pub async fn post<T: Into<Body>>(
        &self,
        action: &str,
        body: Option<T>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let mut builder = self.client.get(self.url.clone());
        builder = self.add_query_params(builder, action);

        if let Some(body_extant) = body {
            builder = builder.body(body_extant)
        }

        let res = builder.send().await;
        return res;
    }
}
