// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use tokio::{
    task,
};

use std::{io, sync::Arc};

mod cert;
mod client;
pub mod http;
mod http1;

#[tokio::main]
async fn main() -> io::Result<()> {
    let cert_data = cert::load_certificate_locations();

    let mut config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_data.certs, cert_data.private_key)
        .unwrap();

    // https://www.iana.org/assignments/tls-extensiontype-values/tls-extensiontype-values.xhtml#alpn-protocol-ids
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config.send_half_rtt_data = true;

    let config = Arc::new(config);

    task::spawn(async move {
        http1::start("127.0.0.1:8080", config).await
    });

    Ok(())
}
