// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::time::{Duration, Instant};

use servente_http_handling::{
    handler::HandlerController,
    ServenteSettings,
};

const INPUT: &str = concat!("GET / HTTP/1.1\r\n",
        "Host: localhost:8080\r\n",
        "User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/111.0\r\n",
        "Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8\r\n",
        "Accept-Language: en-US,en;q=0.5\r\n",
        "Accept-Encoding: gzip, deflate, br\r\n",
        "DNT: 1\r\n",
        "Connection: keep-alive\r\n",
        "Upgrade-Insecure-Requests: 1\r\n",
        "Sec-Fetch-Dest: document\r\n",
        "Sec-Fetch-Mode: navigate\r\n",
        "Sec-Fetch-Site: cross-site\r\n",
        "If-Modified-Since: Thu, 01 Jan 1970 00:00:00 GMT\r\n",
        "\r\n");

async fn handle_exchange() {
    let mut reader = std::io::Cursor::new(INPUT.as_bytes());
    let mut writer = Vec::new();
    servente_http1::handle_exchange(&mut reader, &mut writer, &ServenteSettings{
        handler_controller: HandlerController::new(),
        read_body_timeout: Duration::from_secs(2),
        read_headers_timeout: Duration::from_secs(2),
        middleware: Vec::new(),
    }).await.unwrap();
}

#[tokio::main]
async fn main() {
    let time = Instant::now();
    for _ in 0..1000 {
        handle_exchange().await;
    }
    println!("Finished in {} ms", time.elapsed().as_millis());
}
