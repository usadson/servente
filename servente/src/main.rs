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


    #[cfg(feature = "rustls")]
    let tls_config = create_tls_config_rustls();

    #[cfg(feature = "tls-boring")]
    let tls_config = create_tls_config_boring();

    let mut handler_controller = handler::HandlerController::new();
    example_handlers::register(&mut handler_controller);

    let config = ServenteConfig {
        #[cfg(feature = "rustls")]
        tls_config: std::sync::Arc::new(tls_config),

        #[cfg(feature = "tls-boring")]
        tls_config,

        settings: ServenteSettings {
            handler_controller,
            read_headers_timeout: Duration::from_secs(45),
            read_body_timeout: Duration::from_secs(60),
        },
    };

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

#[cfg(feature = "tls-boring")]
fn create_tls_config_boring() -> boring::ssl::SslAcceptor {
    use boring::ssl;

    let cert_data = servente_self_signed_cert::load_certificate_locations();

    let private_key = boring::pkey::PKey::private_key_from_pkcs8(&cert_data.private_key.0)
        .expect("Failed to load PKCS#8 DER private key");

    let server = ssl::SslMethod::tls_server();
    let mut ssl_builder = boring::ssl::SslAcceptor::mozilla_modern(server)
        .expect("Failed to setup BoringSSL with Mozilla Modern Configuration");

    ssl_builder.set_default_verify_paths().expect("Failed to set default verify paths");
    ssl_builder.set_verify(ssl::SslVerifyMode::NONE);
    ssl_builder.enable_ocsp_stapling();
    ssl_builder.set_alpn_protos(&determine_alpn_protocols_boring()).expect("Failed to set ALPN protocols");
    ssl_builder.set_private_key(&private_key).expect("Failed to set the private key");

    let mut certs = cert_data.certs.iter()
        .map(|der_data| {
            boring::x509::X509::from_der(&der_data.0).unwrap()
        });

    ssl_builder.set_certificate(&certs.next().unwrap())
            .expect("Failed to set the certificate");

    for cert in certs {
        ssl_builder.add_extra_chain_cert(cert)
            .expect("Failed to append chain certificates");
    }

    ssl_builder.build()
}

#[cfg(feature = "rustls")]
fn create_tls_config_rustls() -> rustls::ServerConfig {
    let cert_data = servente_self_signed_cert::load_certificate_locations();

    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_data.certs, cert_data.private_key)
        .expect("Failed to build rustls configuration!");

    // https://www.iana.org/assignments/tls-extensiontype-values/tls-extensiontype-values.xhtml#alpn-protocol-ids
    tls_config.alpn_protocols = determine_alpn_protocols().iter().map(|str| str.as_bytes().to_owned()).collect();
    tls_config.send_half_rtt_data = true;

    #[cfg(feature = "ktls")]
    {
        tls_config.enable_secret_extraction = true;
    }

    tls_config
}

const fn determine_alpn_protocols() -> &'static [&'static str] {
    &[
        #[cfg(feature = "http3")]
        "h3",

        #[cfg(feature = "http2")]
        "h2",

        "http/1.1",
    ]
}

#[cfg(feature = "tls-boring")]
fn determine_alpn_protocols_boring() -> Vec<u8> {
    let protocols = determine_alpn_protocols();
    let length = protocols.iter()
        .map(|str| 1 + str.len())
        .sum();

    let mut result = Vec::with_capacity(length);
    for str in protocols {
        result.push(str.len() as u8);
        result.extend_from_slice(str.as_bytes());
    }

    debug_assert_eq!(result.len(), length);
    result
}
