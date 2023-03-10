// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use tokio::{
    task,
};

use std::{io, sync::Arc, time::Instant};

mod cert;
mod client;
pub mod http;
mod http1;
mod resources;

#[tokio::main]
async fn main() -> io::Result<()> {
    let start = Instant::now();

    let cert_data = cert::load_certificate_locations();

    let mut config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_data.certs, cert_data.private_key)
        .unwrap();

    // https://www.iana.org/assignments/tls-extensiontype-values/tls-extensiontype-values.xhtml#alpn-protocol-ids
    config.alpn_protocols = vec![b"http/1.1".to_vec(), b"h2".to_vec(), b"h3".to_vec()];
    config.send_half_rtt_data = true;

    let config = Arc::new(config);
    let config_v3 = config.clone();

    println!("Loaded after {} ms", start.elapsed().as_millis());

    let join_handle = task::spawn(async move {
        http1::start("127.0.0.1:8080", config).await
    });

    let join_handle_v3 = task::spawn(async move {
        http::v3::start(config_v3).await
    });

    _ = join_handle.await.unwrap();
    _ = join_handle_v3.await.unwrap();

    println!("Stopped after {} ms", start.elapsed().as_millis());
    Ok(())
}
