use tokio::{
    net::{TcpListener, TcpStream},
    task, io::{split, AsyncWriteExt, AsyncReadExt, BufReader, AsyncBufReadExt},
};

use rustls::{
    ServerConfig
};
use tokio_rustls::TlsAcceptor;

use std::{io, sync::Arc};

async fn read_crlf_line<R>(stream: &mut R) -> Result<String, io::Error>
        where R: AsyncBufReadExt + Unpin {
    let mut string = String::new();

    loop {
        let byte = stream.read_u8().await?;
        if byte == '\r' as u8 {
            let byte = stream.read_u8().await?;
            if byte == '\n' as u8 {
                return Ok(string);
            }
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid CRLF"));
        }

        string.push(byte as char);
    }
}

async fn discard_request(stream: &mut TcpStream) -> Result<(), io::Error> {
    let mut buffer = BufReader::new(stream);
    loop {
        let line = read_crlf_line(&mut buffer).await?;
        println!("Discarded: {} ({} len)", line, line.len());
        if line.len() == 0 {
            return Ok(());
        }
    }
}

async fn send_http_upgrade(stream: &mut TcpStream) -> Result<(), io::Error> {
    let body = "HTTPS is required.";
    let message = format!(
        concat!("HTTP/1.1 426 Upgrade Required\r\n",
                "Upgrade: TLS/1.2, HTTP/1.1\r\n",
                "Connection: Upgrade\r\n",
                "Content-Length: {}\r\n",
                "Content-Type: text/plain;charset=utf-8\r\n",
                "\r\n",
                "{}"
        ),
        body.len(), body
    );
    _ = stream.write_all(message.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn process_socket(mut stream: TcpStream, tls_config: Arc<ServerConfig>) {
    println!("Client connected: {}", stream.peer_addr().unwrap());
    let mut buf = [0u8; 4];
    if let Ok(length) = stream.peek(&mut buf).await {
        if length >= 3 && String::from_utf8_lossy(&buf[0..3]) == "GET" {
            if let Err(e) = discard_request(&mut stream).await {
                println!("Client Error discarding non-HTTPS: {:?}", e);
                return;
            }

            send_http_upgrade(&mut stream).await.unwrap();
        }
    }

    let acceptor = TlsAcceptor::from(tls_config);
    let stream = match acceptor.accept(stream).await {
        Ok(stream) => stream,
        Err(e) => {
            println!("Client Error: {:?}", e);
            return;
        }
    };

    let (mut reader, mut writer) = split(stream);

    writer.write_all("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 5\r\n\r\nHello".as_bytes()).await.unwrap();

    println!("Flush");
    _ = writer.flush().await;
}

pub async fn start(address: &str, tls_config: Arc<ServerConfig>) -> io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    println!("Started listening on {}", address);

    loop {
        let (stream, address) = listener.accept().await?;
        let tls_config = Arc::clone(&tls_config);
        task::spawn(async {
            process_socket(stream, tls_config).await;
        });
    }
}
