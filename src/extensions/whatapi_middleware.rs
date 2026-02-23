use http::{Extensions, Method};
use reqwest::{Client, Request, Response};
use reqwest_middleware::{ClientBuilder, Middleware, Next, Result};

struct WhatapiMiddleware {
    authkey: &str,
    passkey: &str,
}

impl Middleware for WhatapiMiddleware {
    // async fn handle(
    //     &self,
    //     req: Request,
    //     extensions: &mut Extensions,
    //     next: Next<'_>,
    // ) -> Result<Response> {
    //     next.run(req, extensions).await
    // }

    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response>
    {
        req.query_mut()
        next.run(req, extensions).await
    }
}
