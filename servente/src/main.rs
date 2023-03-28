// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

// #![warn(
//     missing_docs,
//     clippy::missing_docs_in_private_items,
//     clippy::missing_errors_doc,
//     clippy::missing_panics_doc
// )]

use servente_resources::cache;
use tokio::task;

use std::{io, sync::Arc, time::Instant, env::current_dir};

mod abnf;
mod cert;
mod client;
mod example_handlers;
mod handler;
pub mod http;
mod platform;

#[derive(Clone)]
pub struct ServenteConfig {
    pub tls_config: Arc<rustls::ServerConfig>,
    pub handler_controller: handler::HandlerController,
}

unsafe impl Send for ServenteConfig {}
unsafe impl Sync for ServenteConfig {}

#[tokio::main]
async fn main() -> io::Result<()> {
    let start = Instant::now();

    let wwwroot_path = current_dir().unwrap().join("wwwroot");

    let cert_data = cert::load_certificate_locations();

    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_data.certs, cert_data.private_key)
        .expect("Failed to build rustls configuration!");

    // https://www.iana.org/assignments/tls-extensiontype-values/tls-extensiontype-values.xhtml#alpn-protocol-ids
    tls_config.alpn_protocols = vec![
        #[cfg(feature = "http3")]
        b"h3".to_vec(),

        #[cfg(feature = "http2")]
        b"h2".to_vec(),

        b"http/1.1".to_vec()
    ];
    tls_config.send_half_rtt_data = true;

    #[cfg(feature = "ktls")]
    {
        tls_config.enable_secret_extraction = true;
    }

    let mut handler_controller = handler::HandlerController::new();
    example_handlers::register(&mut handler_controller);

    let config = ServenteConfig {
        tls_config: Arc::new(tls_config),
        handler_controller,
    };

    #[cfg(feature = "http3")]
    let config_v3 = config.clone();

    println!("Loaded after {} ms", start.elapsed().as_millis());

    let join_handle = task::spawn(async move {
        http::v1::start("127.0.0.1:8080", config).await
    });

    #[cfg(feature = "http3")]
    let join_handle_v3 = task::spawn(async move {
        http::v3::start(config_v3.tls_config).await
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
