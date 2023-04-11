// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

#![allow(dead_code)]

use std::{
    io,
    net::{
        IpAddr,
        SocketAddr, Ipv4Addr,
    },
    sync::Arc,
};

use quinn::{SendStream, RecvStream};

mod static_table;

async fn handle_connection(connection: quinn::Connecting) -> io::Result<()> {
    let connection = connection.await?;

    let protocol = connection.handshake_data().unwrap()
        .downcast::<quinn::crypto::rustls::HandshakeData>().unwrap()
        .protocol.map_or_else(|| "".into(), |x| String::from_utf8_lossy(&x).into_owned());

        println!("[QUIC] New connection from {} using protocol {}", connection.remote_address(), protocol);

    loop {
        let stream = connection.accept_bi().await?;
        let fut = handle_request(stream.0, stream.1);
        tokio::task::spawn(async move {
            _ = fut.await;
        });
    }
}

async fn handle_request(send_stream: SendStream, recv_stream: RecvStream) -> io::Result<()> {
    let mut recv_buffer = [0u8; 1024];
    let mut send_buffer = [0u8; 1024];

    let mut recv_stream = recv_stream;
    let mut send_stream = send_stream;

    let mut request = String::new();

    Ok(())
}

pub async fn start(tls_config: Arc<rustls::ServerConfig>) -> io::Result<()> {
    let config = quinn::ServerConfig::with_crypto(tls_config);
    let endpoint = quinn::Endpoint::server(config, SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080))
        .unwrap();

    while let Some(connection) = endpoint.accept().await {
        tokio::task::spawn(async move {
            _ = handle_connection(connection).await;
        });
    }

    Ok(())
}
