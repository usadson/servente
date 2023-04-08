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
