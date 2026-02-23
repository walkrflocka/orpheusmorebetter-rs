use reqwest::{Client, Error, Request, Url};

pub struct AjaxSession<'a> {
    authkey: &'a str,
    url: Url,
    client: Client,
}

impl AjaxSession {
    pub fn new(client: Client, authkey: &str, base_url: Url) -> Self {
        AjaxSession {
            authkey,
            url: base_url
                .join("ajax.php")
                .expect("Failed to construct Ajax endpoint"),
            client: client,
        }
    }

    fn add_query_params(
        &self,
        builder: reqwest::RequestBuilder,
        action: &str,
    ) -> reqwest::RequestBuilder {
        builder.query(&[("authkey", self.authkey), ("action", action)])
    }

    pub async fn get(&self, action: &str) {
        let builder = self.client.get(url);
        self.add_query_params(builder, action).send().await
    }

    pub async fn post(&self, action)
}
