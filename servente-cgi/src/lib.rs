// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use async_trait::async_trait;

use servente_http::{
    RequestTarget, Response, StatusCode
};

use servente_http_handling::{
    Middleware,
    middleware::{
        ExchangeState,
        MiddlewareError,
    },
};
use servente_resources::MediaType;

#[derive(Copy, Clone, Debug)]
pub struct CgiMiddleware {

}

impl CgiMiddleware {
    /// Creates a new instance of CgiMiddleware.
    pub fn new() -> Self {
        Self {
        }
    }
}

#[async_trait]
impl Middleware for CgiMiddleware {
    fn debug_identifier(&self) -> &str {
        "servente_cgi::CgiMiddleware"
    }

    async fn invoke(&mut self, state: &mut ExchangeState) -> Result<(), MiddlewareError> {
        if let RequestTarget::Origin { path, .. } = &state.request.target {
            if path == "/cgi/test" {
                state.response = Response::with_status_and_string_body(StatusCode::BadGateway, "CGI Gateway Test");
                state.response.headers.set_content_type(MediaType::HTML);
            } else if path.starts_with("/cgi") {
                return Err(anyhow::anyhow!("Not a CGI script").into());
            }
        }
        Ok(())
    }
}
