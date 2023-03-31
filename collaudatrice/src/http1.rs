// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{
    net::SocketAddr,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};

use futures::{
    Future,
    stream::FuturesUnordered,
    StreamExt,
};
use tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
        split,
    },
    net::TcpStream,
};

use crate::Configuration;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TestName(&'static str);

#[derive(Clone, Debug)]
struct TestParameters {
    rustls_config: Arc<rustls::ClientConfig>,
    domain: Arc<str>,
    address: SocketAddr
}

#[derive(Debug)]
#[allow(dead_code)]
enum TestResult {
    Passed,
    Failed {
        message: Vec<anyhow::Error>
    },
    TimedOut,
}

async fn invoke_task<F>(name: TestName, f: F) -> (TestName, TestResult)
        where F: Future<Output = anyhow::Result<TestResult>> {
    match tokio::time::timeout(Duration::from_secs(10), f).await {
        Ok(result) => (name, match result {
            Ok(e) => e,
            Err(e) => TestResult::Failed {
                message: vec![e]
            }
        }),
        Err(_) => (name, TestResult::TimedOut)
    }
}

pub async fn run(config: &Configuration, address: SocketAddr) {
    let parameters = TestParameters {
        rustls_config: Arc::clone(&config.rustls_client_config),
        domain: Arc::from(config.args.host.as_str()),
        address,
    };

    let mut tasks = FuturesUnordered::new();
    tasks.push(invoke_task(TestName("Simple GET request"), test_get(parameters.clone())));

    let start_time = Instant::now();
    while let Some((name, result)) = tasks.next().await {
        println!("  {}: {result:?}", name.0);
    }

    println!("Completed all work in {} ms", start_time.elapsed().as_millis());
}

async fn simple_exchange(parameters: TestParameters, input: &[u8]) -> anyhow::Result<Vec<u8>> {
    let stream = TcpStream::connect(parameters.address).await?;
    let connection = tokio_rustls::TlsConnector::from(parameters.rustls_config)
            .connect(rustls::ServerName::try_from(parameters.domain.as_ref())?, stream).await?;

    let (mut reader, mut writer) = split(connection);

    writer.write_all(input).await?;
    writer.flush().await?;
    drop(writer);

    let mut data = Vec::new();
    _ = reader.read_to_end(&mut data).await?;

    Ok(data)
}

async fn test_get(parameters: TestParameters) -> anyhow::Result<TestResult> {
    let domain = Arc::clone(&parameters.domain);
    simple_exchange(parameters, format!("GET / HTTP/1.1\r\nHost: {}\r\n\r\n", domain.as_ref()).as_bytes()).await?;

    Ok(TestResult::Passed)
}
