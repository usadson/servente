// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{
    net::SocketAddr,
    sync::Arc,
};

use anyhow::{bail, anyhow};
use tokio::{net::TcpStream, io::{AsyncWriteExt, AsyncReadExt, BufReader}};

use crate::{Configuration, io::read_to_crlf};

#[derive(Clone, Debug)]
pub struct ServerAnalysis {
    pub server_product_name: String,
    pub ipv4_address: Option<SocketAddr>,
    pub ipv6_address: Option<SocketAddr>,
}

/// Find out some details about the server.
pub async fn analyze_server(config: &Configuration) -> anyhow::Result<ServerAnalysis> {
    let host = format!("{}:{}", config.args.host, config.args.port);
    let addresses = tokio::net::lookup_host(&host).await
        .map_err(|e| anyhow::Error::from(e).context(format!("Failed to lookup host: {host}")))?;

    let mut analysis = ServerAnalysis {
        server_product_name: String::new(),
        ipv4_address: None,
        ipv6_address: None
    };

    let mut failed_requests = Vec::new();
    for address in addresses {
        match &address {
            SocketAddr::V4(_) if analysis.ipv4_address.is_none() => {
                match find_server_product_name(config, address).await {
                    Ok(name) => {
                        analysis.server_product_name = name;
                        analysis.ipv4_address = Some(address);
                    }
                    Err(e) => {
                        failed_requests.push(e);
                        failed_requests.push(anyhow!("Failed to connect to {address}"));
                    }
                }
            }
            SocketAddr::V6(_) if analysis.ipv6_address.is_none() => {
                match find_server_product_name(config, address).await {
                    Ok(name) => {
                        analysis.server_product_name = name;
                        analysis.ipv6_address = Some(address);
                    }
                    Err(e) => {
                        failed_requests.push(e);
                        failed_requests.push(anyhow!("Failed to connect to {address}"));
                    }
                }
            }
            _ => continue,
        }
    }

    if analysis.ipv4_address.is_none() && analysis.ipv6_address.is_none() {
        if !failed_requests.is_empty() {
            let mut error = anyhow!("");
            for failed_request in failed_requests {
                error = error.context(failed_request);
            }

            return Err(error.context("Failed to connect to host(s)"));
        }

        bail!("Failed to lookup host, neither IPv4 nor IPv6 entry found for: {host}");
    }

    Ok(analysis)
}

async fn find_server_product_name(config: &Configuration, address: SocketAddr) -> anyhow::Result<String> {
    let stream = TcpStream::connect(address).await?;

    let mut connection = tokio_rustls::TlsConnector::from(Arc::clone(&config.rustls_client_config))
            .connect(rustls::ServerName::try_from(config.args.host.as_ref())?, stream).await?;


    connection.write_all(
        format!(
            concat!(
                "GET / HTTP/1.1\r\n",
                "Host: {}\r\n",
                "Connection: close\r\n",
                "\r\n"
            ),
            config.args.host
        ).as_bytes()
    ).await?;

    connection.flush().await?;

    let mut reader = BufReader::new(connection);
    let data = read_data(&mut reader).await?;
    if data.is_empty() {
        bail!("Failed to send request: returned empty data");
    }

    if !data[0].starts_with("HTTP/1.1") {
        bail!("Failed to send request: response doesn't send HTTP/1.1 response");
    }

    for line in data {
        let header_start = "Server: ";
        if let Some(value) = line.strip_prefix(header_start) {
            return Ok(value.to_owned());
        }
    }

    Ok(String::from("[unknown]"))
}

async fn read_data<R>(reader: &mut R) -> anyhow::Result<Vec<String>>
        where R: AsyncReadExt + Unpin {
    let mut result = Vec::new();

    loop {
        let line = read_to_crlf(reader).await?;
        if line.is_empty() {
            break;
        }

        result.push(line);
    }

    Ok(result)
}
