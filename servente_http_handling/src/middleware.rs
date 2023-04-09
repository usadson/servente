// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use async_trait::async_trait;
use dyn_clone::DynClone;

use servente_http::{
    Request,
    Response,
};

/// The state of a request that is being handled, and the response that will be
/// sent accordingly.
pub struct ExchangeState<'a> {
    /// The request that's being handled.
    pub request: &'a Request,

    /// The response is being generated.
    pub response: Response,
}

/// Middleware is a step in the handling of a process.
///
/// `Middleware` must be clone'able to ensure multiple requests can be handled
/// concurrently.
#[async_trait]
pub trait Middleware: DynClone + Send + Sync {
    /// The name of the middleware to identity the source of errors when
    /// displaying them in the error body.
    fn debug_identifier(&self) -> &str;

    /// Asynchronously handle the request by invoking this function. Note that
    /// the behavior of invocations are order-dependent, meaning that middleware
    /// down the line can substantially change the contents of the response.
    ///
    /// Protocol-dependent behavior is managed by the protocol suites like
    /// `servente_http1`, meaning that these behaviors can only be communicated
    /// by using the correct structure, or sometimes not at all, e.g.
    /// [Transfer-Encoding](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Transfer-Encoding).
    async fn invoke(&mut self, state: &mut ExchangeState) -> Result<(), MiddlewareError>;
}

/// `MiddlewareError` is an error that can generate during the invocation of a
/// middleware component.
#[derive(Debug)]
pub enum MiddlewareError {
    /// Middleware failed, but the condition is not unrecoverable.
    /// `anyhow::Error` is contained for debugging purposes, but is
    /// ignored in release builds.
    RecoverableError(anyhow::Error),

    /// Unrecoverable error.
    UnrecoverableError(anyhow::Error),
}

impl From<anyhow::Error> for MiddlewareError {
    fn from(value: anyhow::Error) -> Self {
        MiddlewareError::UnrecoverableError(value)
    }
}
