// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

pub mod http1;

use async_trait::async_trait;

#[async_trait]
pub trait WebSocketConnection {
    async fn close(&mut self);
    async fn send_message_binary(&mut self, data: &[u8]) -> Result<(), std::io::Error>;
    async fn send_message_text(&mut self, data: &str) -> Result<(), std::io::Error>;
}

#[async_trait]
pub trait WebSocketFrameHandler {
    async fn handle_close(&mut self);

    async fn handle_message_binary(&mut self, data: Vec<u8>) -> Result<(), anyhow::Error>;

    async fn handle_message_text(&mut self, data: Vec<u8>) -> Result<(), anyhow::Error>;
}

#[async_trait]
pub trait WebSocketFrameHandlerProvider {
    async fn provide(&mut self, connection: Box<dyn WebSocketConnection>) -> Box<dyn WebSocketFrameHandler>;
}

#[derive(Default)]
pub struct WebSocketRegistry {
    map: hashbrown::HashMap<String, Box<dyn WebSocketFrameHandlerProvider>>,
}

impl WebSocketRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, name: String, provider: Box<dyn WebSocketFrameHandlerProvider>) {
        self.map.insert(name, provider);
    }
}
