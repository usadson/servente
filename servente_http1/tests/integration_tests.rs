// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! Integration tests for the HTTP/1.1 server.

use std::{
    fs::DirBuilder,
    time::Duration,
    sync::Arc, process::{Command, Output},
};

use servente_http_handling::{ServenteConfig, handler};
use tokio::{task::AbortHandle, time::{sleep, timeout}};

fn setup_configuration() -> ServenteConfig {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let wwwroot_path = temp_dir.path().join("wwwroot");
    DirBuilder::new().create(&wwwroot_path).unwrap();

    let cert_data = servente_self_signed_cert::load_certificate_locations();

    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_data.certs, cert_data.private_key)
        .expect("Failed to build rustls configuration!");

    tls_config.alpn_protocols = vec!["http/1.1".into()];

    let handler_controller = handler::HandlerController::new();

    ServenteConfig {
        tls_config: Arc::new(tls_config),
        handler_controller,
        read_headers_timeout: Duration::from_secs(10),
        read_body_timeout: Duration::from_secs(10),
    }
}

async fn start_server_in_background() -> AbortHandle {
    let config = setup_configuration();
    tokio::task::spawn(async {
        servente_http1::start("127.0.0.1:40626", config).await
    }).abort_handle()
}

#[tokio::test]
async fn test_curl_integration() {
    start_server_in_background().await;
    tokio::task::yield_now().await;

    async fn run_curl() -> Result<Output, tokio::time::error::Elapsed> {
        let fut = tokio::task::spawn_blocking(|| {
            Command::new("curl")
                .arg("-k") // Insecure, since the certificate is self-signed
                .arg("-i") // Include data
                .arg("-v") // Verbose
                .arg("https://localhost:40626/")
                .spawn()
                .expect("Failed to invoke curl")
                .wait_with_output()
                .expect("Failed to wait_with_output")
        });

        timeout(Duration::from_secs(2), async {
            fut.await.unwrap()
        }).await
    }

    let mut curl_output = run_curl().await;
    let mut attempt = 0;
    while curl_output.is_err() || !curl_output.as_ref().unwrap().status.success() {
        attempt += 1;
        if attempt == 4 {
            break;
        }

        sleep(Duration::from_secs(2)).await;

        curl_output = run_curl().await;
    }

    let curl_output = curl_output.expect("cURL invocation timed out");

    assert!(curl_output.status.success(), "Integration test with cURL failed: \nErr: {}\nOut: {}",
        String::from_utf8_lossy(&curl_output.stderr), String::from_utf8_lossy(&curl_output.stdout));
}
