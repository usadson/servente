// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::time::Duration;

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

    async fn invoke_cgi<'a>(&mut self, state: &mut ExchangeState<'a>, path: &str) -> Result<(), anyhow::Error> {
        _ = path;

        let process = match tokio::process::Command::new("wwwroot/test.pl")
            //.env("key", val)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn() {
            Ok(process) => process,
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound => {
                        state.response = Response::with_status_and_string_body(StatusCode::NotFound, "Not Found");
                        return Ok(());
                    }
                    _ => ()
                }

                #[cfg(not(debug_assertions))]
                return Err(e.into());

                #[cfg(debug_assertions)]
                {
                    state.response = Response::with_status_and_string_body(StatusCode::InternalServerError, format!("Error: {e}"));
                    return Ok(());
                }
            }
        };

        match tokio::time::timeout(Duration::from_secs(10), process.wait_with_output()).await {
            Ok(result) => match result {
                Ok(e) => {
                    state.response = Response::with_status(StatusCode::Ok);
                    let mut stdout = std::io::Cursor::new(&e.stdout);
                    let headers = servente_http1::read::read_headers(&mut stdout).await
                        .map_err(|e| anyhow::anyhow!(format!("{e:?}")))?;
                    for header in headers.into_iter() {
                        state.response.headers.append_or_override(header.0, header.1);
                    }
                    state.response.body = Some(servente_http::BodyKind::Bytes(e.stdout[(stdout.position() as usize)..].into()));
                }
                Err(e) => {
                    state.response = if cfg!(debug_assertions) {
                        Response::with_status_and_string_body(StatusCode::InternalServerError, format!("Error: {e}"))
                    } else {
                        Response::with_status_and_string_body(StatusCode::InternalServerError, "Internal Server Error")
                    };

                    return Ok(());
                }
            }
            Err(_) => {
                state.response = Response::with_status_and_string_body(StatusCode::GatewayTimeout, "Gateway Timeout");
                return Ok(());
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Middleware for CgiMiddleware {
    fn debug_identifier(&self) -> &str {
        "servente_cgi::CgiMiddleware"
    }

    async fn invoke(&mut self, state: &mut ExchangeState) -> Result<(), MiddlewareError> {
        if let RequestTarget::Origin { path, .. } = &state.request.target {
            if path == "/test.pl" {
                self.invoke_cgi(state, path).await?;
            }
        }
        Ok(())
    }
}
