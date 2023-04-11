// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This module contains the implementation of WebSockets for the HTTP/1.1
//! protocol.

use async_trait::async_trait;
use tokio::io::{
    AsyncBufReadExt,
    AsyncWriteExt,
};

use super::WebSocketConnection;

pub struct Http1WebSocketConnection<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> Http1WebSocketConnection<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
        }
    }
}

#[async_trait]
impl<R, W> WebSocketConnection for Http1WebSocketConnection<R, W>
        where R: AsyncBufReadExt + Unpin + Send,
              W: AsyncWriteExt + Unpin + Send {
    async fn close(&mut self) {

    }

    async fn send_message_binary(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        Ok(())
    }

    async fn send_message_text(&mut self, data: &str) -> Result<(), std::io::Error> {
        Ok(())
    }
}
