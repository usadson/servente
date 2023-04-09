// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

// #![warn(
//     missing_docs,
//     clippy::missing_docs_in_private_items,
//     clippy::missing_errors_doc,
//     clippy::missing_panics_doc
// )]

use servente_http_handling::{handler, ServenteConfig, ServenteSettings};
use servente_resources::cache;
use tokio::task;

use std::{io, time::{Instant, Duration}, env::current_dir};

mod example_handlers;

#[cfg(not(feature = "io_uring"))]
#[tokio::main]
async fn main() -> io::Result<()> {
    begin().await
}

#[cfg(feature = "io_uring")]
fn main() -> io::Result<()> {
    tokio_uring::start(begin())
}

async fn begin() -> io::Result<()> {
    let start = Instant::now();

    let wwwroot_path = current_dir().unwrap().join("wwwroot");

    let mut handler_controller = handler::HandlerController::new();
    example_handlers::register(&mut handler_controller);

    let middleware = Vec::new();

    #[cfg(feature = "cgi")]
    let mut middleware = middleware;

    #[cfg(feature = "cgi")]
    setup_cgi(&mut middleware);

    let config = ServenteConfig::new().build(ServenteSettings {
        handler_controller,
        read_headers_timeout: Duration::from_secs(45),
        read_body_timeout: Duration::from_secs(60),
        middleware,
    });

    #[cfg(feature = "http3")]
    let config_v3 = config.clone();

    println!("Loaded after {} ms", start.elapsed().as_millis());

    let join_handle = task::spawn(async move {
        servente_http1::start("127.0.0.1:8080", config).await
    });

    #[cfg(feature = "http3")]
    let join_handle_v3 = task::spawn(async move {
        servente_http3::start(config_v3.tls_config).await
    });

    let wwwroot_path_cacher = wwwroot_path.clone();
    let join_handle_cache = task::spawn(async move {
        cache::start(&wwwroot_path_cacher).await
    });

    if let Err(e) = join_handle.await.unwrap() {
        println!("Server error (HTTP/1.1): {}", e);
    }

    #[cfg(feature = "http3")]
    {
        _ = join_handle_v3.await.unwrap();
    }

    join_handle_cache.abort();

    println!("Stopped after {} ms", start.elapsed().as_millis());
    Ok(())
}

#[cfg(feature = "cgi")]
fn setup_cgi(middleware: &mut Vec<std::sync::Arc<dyn servente_http_handling::Middleware>>) {
    use std::sync::Arc;

    middleware.push(Arc::new(servente_cgi::CgiMiddleware::new()));
}
