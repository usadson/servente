// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! The `servente-cgi` crate implements **Common Gateway Interface** logic for
//! the Servente webserver.
//!
//! # References
//! * Robinson, D. and K. Coar, "The Common Gateway Interface (CGI) Version 1.1",
//!   RFC 3875, DOI 10.17487/RFC3875, October 2004, <https://www.rfc-editor.org/info/rfc3875>.

use std::{time::Duration, env::current_dir};

use async_trait::async_trait;

use servente_http::{
    HeaderName,
    Request,
    RequestTarget,
    Response,
    StatusCode,
};

use servente_http_handling::{
    middleware::{
        ExchangeState,
        MiddlewareError,
    },
    Middleware,
};

use tokio::io::AsyncWriteExt;

#[derive(Copy, Clone, Debug)]
/// The middleware that supports interacting with CGI scripts by handling
/// requests that depend on CGI behavior.
///
/// It implements the [`Middleware`] trait, which detects CGI invocations and
/// invokes those commands instead of returning the contents of the script.
pub struct CgiMiddleware {
}

fn set_command_environment_variables(request: &Request, command: &mut tokio::process::Command) {
    command.env("GATEWAY_INTERFACE", "CGI/1.1")
        .env("REQUEST_METHOD", request.method.as_string())
        .env("SERVER_NAME", "localhost")
        .env("SERVER_PORT", "8080")
        .env("SERVER_PROTOCOL", request.version.to_http_version())
        .env("SERVER_SOFTWARE", "Servente");

    if let RequestTarget::Origin { path, query } = &request.target {
        command.env("QUERY_STRING", query)
            .env("PATH_INFO", path);
    }

    // TODO set [REMOTE_ADDR](https://www.rfc-editor.org/rfc/rfc3875.html#section-4.1.8)

    if let Some(body) = &request.body {
        match body {
            servente_http::BodyKind::Bytes(data) => _ = command.env("CONTENT_LENGTH", format!("{}", data.len())),
            servente_http::BodyKind::String(data) => _ = command.env("CONTENT_LENGTH", format!("{}", data.len())),
            _ => ()
        }
    }

    for (header_name, header_value) in request.headers.iter() {
        match header_name {
            HeaderName::ContentType => _ = command.env("CONTENT_TYPE", header_value.as_str_may_convert().as_ref()),
            _ => (),
        }
    }
}

impl CgiMiddleware {
    /// Creates a new instance of CgiMiddleware.
    pub fn new() -> Self {
        Self {
        }
    }

    /// Creates a [`tokio::process::Command`] which can be used to `spawn` the
    /// script, with the correct environment already defined.
    fn create_cgi_script_command(&self, request: &Request) -> Option<tokio::process::Command> {
        let RequestTarget::Origin { path, .. } = &request.target else {
            return None;
        };

        let wwwroot = current_dir().ok()?.join("wwwroot");
        let script_path = match servente_http_handling::find_request_path_in_wwwroot(&wwwroot, &path) {
            Ok(path) => path,
            Err(_) => return None,
        };

        let script_file_name = script_path.file_name()?;

        let mut command = tokio::process::Command::new(format!("./{}", script_file_name.to_string_lossy()));

        if let Some(parent_dir) = script_path.parent() {
            if let Some(parent_dir) = parent_dir.canonicalize().ok() {
                command.current_dir(parent_dir);
            } else {
                command.current_dir(parent_dir);
            }
        } else {
            return None;
        }

        set_command_environment_variables(request, &mut command);

        command
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped());

        Some(command)
    }

    /// Creates a process for the given command.
    async fn create_process_from_command<'a>(&self,
            mut command: tokio::process::Command,
            state: &mut ExchangeState<'a>
        ) -> Result<Option<tokio::process::Child>, anyhow::Error> {
        match command.spawn() {
            Ok(process) => Ok(Some(process)),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        state.response = Response::with_status_and_string_body(StatusCode::NotFound, format!("Nfot Found: {e:?}"));
                        return Ok(None);
                    }
                    std::io::ErrorKind::NotFound => {
                        state.response = Response::with_status_and_string_body(StatusCode::NotFound, format!("Nfot Found: {e:?}"));
                        return Ok(None);
                    }
                    _ => ()
                }

                #[cfg(not(debug_assertions))]
                return Err(e.into());

                #[cfg(debug_assertions)]
                {
                    state.response = Response::with_status_and_string_body(StatusCode::InternalServerError, format!("Error: {e}"));
                    return Ok(None);
                }
            }
        }
    }

    /// The entrypoint for the CGI middleware.
    async fn invoke_cgi<'a>(&self, state: &mut ExchangeState<'a>) -> Result<(), anyhow::Error> {
        let Some(command) = self.create_cgi_script_command(state.request) else {
            return Ok(())
        };

        let Some(mut process) = self.create_process_from_command(command, state).await? else {
            return Ok(());
        };

        self.pass_request_body_to_process(&mut process, &state.request).await?;

        self.process_cgi_process(process,
            state).await
    }

    /// The entrypoint for the CGI middleware.
    async fn pass_request_body_to_process(&self, process: &mut tokio::process::Child, request: &Request) -> Result<(), std::io::Error> {
        let Some(body) = &request.body else {
            return Ok(());
        };

        let Some(stdin) = &mut process.stdin else {
            return Ok(());
        };

        match body {
            servente_http::BodyKind::Bytes(data) => stdin.write_all_buf(&mut std::io::Cursor::new(data)).await,
            servente_http::BodyKind::String(data) => stdin.write_all_buf(&mut std::io::Cursor::new(data)).await,
            _ => Ok(())
        }
    }

    /// Processes the CGI process, parsing it's content and producing a response
    /// for that output. It also has a timeout to prevent the script from
    /// running too long, e.g. due to a deadlock.
    async fn process_cgi_process<'a>(&self, process: tokio::process::Child, state: &mut ExchangeState<'a>) -> Result<(), anyhow::Error> {
        let Ok(result) = tokio::time::timeout(Duration::from_secs(10), process.wait_with_output()).await else {
            // Timed out.
            state.response = Response::with_status_and_string_body(StatusCode::GatewayTimeout, "Gateway Timeout");

            // TODO is the child killed?
            return Ok(());
        };

        match result {
            Ok(e) => return self.produce_response_for_cgi_output(state, &e.stdout).await,
            Err(e) => {
                state.response = if cfg!(debug_assertions) {
                    Response::with_status_and_string_body(StatusCode::InternalServerError, format!("Error: {e}"))
                } else {
                    Response::with_status_and_string_body(StatusCode::InternalServerError, "Internal Server Error")
                };

                Ok(())
            }
        }
    }

    /// Processes the CGI process output, parsing it's content and producing a
    /// response accordingly.
    async fn produce_response_for_cgi_output<'a>(&self, state: &mut ExchangeState<'a>, stdout: &Vec<u8>) -> Result<(), anyhow::Error> {
        state.response = Response::with_status(StatusCode::Ok);
        let mut stdout_cursor = std::io::Cursor::new(&stdout);

        // TODO newlines in CGI are different from newlines in HTTP/1, since
        //      they *may* be platform-dependent. As an example, Windows uses
        //      CRLF, macOS CR and Linux commonly LF.
        let headers = servente_http1::read::read_headers(&mut stdout_cursor).await
            .map_err(|e| anyhow::anyhow!(format!("{e:?}")))?;

        for (header_name, header_value) in headers.into_iter() {
            match header_name.class() {
                servente_http::HeaderNameClass::CgiExtension => {
                    // Ignore CGI-specific header fields.
                    println!("[CGI] Ignoring extension field: \"{}\" => {:?}", header_name.to_string_h1(), header_value);
                }
                servente_http::HeaderNameClass::ConnectionSpecific => (),
                servente_http::HeaderNameClass::Other => {
                    state.response.headers.append_or_override(header_name, header_value);
                }
            }
        }

        if !state.response.headers.contains(&HeaderName::CacheControl) {
            state.response.headers.append_or_override(HeaderName::CacheControl, "no-store".into());
        }

        state.response.body = Some(servente_http::BodyKind::Bytes(stdout[(stdout_cursor.position() as usize)..].into()));
        Ok(())
    }
}

#[async_trait]
impl Middleware for CgiMiddleware {
    fn debug_identifier(&self) -> &str {
        "servente_cgi::CgiMiddleware"
    }

    async fn invoke(&mut self, state: &mut ExchangeState) -> Result<(), MiddlewareError> {
        self.invoke_cgi(state).await?;
        Ok(())
    }
}
