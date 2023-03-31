// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{
    io,
    net::{
        IpAddr,
        SocketAddr, Ipv4Addr, Ipv6Addr,
    },
    sync::{Arc, Weak},
};

use hashbrown::HashMap;
use quinn::{
    RecvStream,
    SendStream,
    VarInt,
};
use tokio::{select, io::AsyncWriteExt};
use tokio::sync::RwLock;

use crate::qpack_stream::{
    DynamicTable,
    QpackEncoderReceiveStream,
};

use self::io_extensions::*;

mod constants;
mod error;
mod io_extensions;
mod qpack_stream;
mod settings;
mod static_table;

use error::*;
use settings::*;

#[derive(Clone, Debug)]
pub(crate) struct ConnectionHandle {
    /// The ID of the connection
    id: usize,
    notifier: tokio::sync::mpsc::Sender<ConnectionNotification>,
    connection_info: Weak<RwLock<ConnectionInfo>>,
    dynamic_table: Weak<RwLock<qpack_stream::DynamicTable>>,
}

struct ConnectionInfo {
    client_has_opened_qpack_encoder_stream: bool,
    decoder_send_stream_to_take: Option<SendStream>,
}

impl ConnectionInfo {
    pub fn create(decoder_send_stream_to_take: SendStream) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            client_has_opened_qpack_encoder_stream: false,
            decoder_send_stream_to_take: Some(decoder_send_stream_to_take),
        }))
    }
}

pub(self) enum ConnectionNotification {
    ConnectionError(ErrorCode),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Frame {
    Data {
        payload: Vec<u8>
    },
    Headers {
        payload: Vec<u8>
    },
    CancelPush {
        push_id: PushId,
    },
    Settings {
        settings: HashMap<SettingKind, usize>,
    },
    PushPromise {
        push_id: PushId,
        payload: Vec<u8>,
    },
    GoAway {
        stream_or_push_id: StreamOrPushId,
    },
    Origin {
        entries: Vec<String>,
    },
    MaxPushId {
        push_id: PushId,
    },
    Metadata {

    },
    /// # References
    /// * [RFC 9218 Section 7.2](https://www.rfc-editor.org/rfc/rfc9218.html#name-the-priority_update-frame)
    PriorityUpdate {
        origin: PriorityUpdateOrigin,
        stream_or_push_id: StreamOrPushId,
        value: String,
    },
    Unknown {
        frame_type: usize,
        payload: Vec<u8>,
    }
}

impl Frame {
    pub const fn frame_type(&self) -> usize {
        match self {
            Frame::Data { .. } => FRAME_TYPE_DATA,
            Frame::Headers { .. } => FRAME_TYPE_HEADERS,
            Frame::CancelPush { .. } => FRAME_TYPE_CANCEL_PUSH,
            Frame::Settings { .. } => FRAME_TYPE_SETTINGS,
            Frame::PushPromise { .. } => FRAME_TYPE_PUSH_PROMISE,
            Frame::GoAway { .. } => FRAME_TYPE_GOAWAY,
            Frame::Origin { .. } => FRAME_TYPE_ORIGIN,
            Frame::MaxPushId { .. } => FRAME_TYPE_MAX_PUSH_ID,
            Frame::Metadata { .. } => FRAME_TYPE_METADATA,
            Frame::PriorityUpdate { origin, .. } => match origin {
                PriorityUpdateOrigin::RequestStream => FRAME_TYPE_PRIORITY_UPDATE_REQUEST,
                PriorityUpdateOrigin::PushStream => FRAME_TYPE_PRIORITY_UPDATE_PUSH,
            }
            Frame::Unknown { frame_type, .. } => *frame_type,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Frame::Data { payload } => payload.len(),
            Frame::Headers { payload } => payload.len(),
            Frame::CancelPush { push_id } => variable_integer_encoded_length(push_id.0) as usize,
            Frame::Settings { settings } => settings
                    .iter()
                    .map(|(key, value)| {
                        (variable_integer_encoded_length(*key as usize) + variable_integer_encoded_length(*value)) as usize
                    })
                    .sum(),
            Frame::PushPromise { push_id, payload } => variable_integer_encoded_length(push_id.0) as usize + payload.len(),
            Frame::GoAway { stream_or_push_id } => variable_integer_encoded_length(Into::<usize>::into(*stream_or_push_id)) as usize,
            Frame::Origin { .. } => todo!(),
            Frame::MaxPushId { push_id } => variable_integer_encoded_length(push_id.0) as usize,
            Frame::Metadata { .. } => todo!(),
            Frame::PriorityUpdate { .. } => todo!(),
            Frame::Unknown { payload, .. } => payload.len(),
        }
    }
}

async fn handle_connection(connection: quinn::Connecting) -> io::Result<()> {
    let connection = connection.await?;

    let protocol = connection.handshake_data().unwrap()
        .downcast::<quinn::crypto::rustls::HandshakeData>().unwrap()
        .protocol.map_or_else(|| "".into(), |x| String::from_utf8_lossy(&x).into_owned());

    println!("[QUIC] New connection from {} using protocol {}", connection.remote_address(), protocol);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(16384);

    let decoder_send_stream_to_take = connection.open_uni().await?;

    let connection_info = ConnectionInfo::create(decoder_send_stream_to_take);
    let dynamic_table = Arc::new(RwLock::new(DynamicTable::new()));

    let connection_handle = ConnectionHandle {
        id: connection.stable_id(),
        notifier: sender,
        connection_info: Arc::downgrade(&Arc::clone(&connection_info)),
        dynamic_table: Arc::downgrade(&Arc::clone(&dynamic_table))
    };

    let mut control_send_stream = match connection.open_uni().await {
        Ok(control_send_stream) => control_send_stream,
        Err(e) => {
            println!("[HTTP/3] Failed to open the uni control stream: {e:?}");
            return Err(io::Error::new(io::ErrorKind::ConnectionReset, e.to_string()));
        }
    };

    if let Err(e) = control_send_stream.write_stream_header(UnidirectionalStreamType::Control).await {
        println!("[HTTP/3] Failed to open control stream for settings");
        return Err(io::Error::new(io::ErrorKind::Other, format!("{e:?}")));
    }
    if let Err(e) = control_send_stream.write_frame(Frame::Settings { settings: HashMap::new() }).await {
        println!("[HTTP/3] Failed to write settings: {e:?}");
        return Err(io::Error::new(io::ErrorKind::Other, format!("{e:?}")));
    }

    _ = control_send_stream.flush().await?;

    loop {
        select! {
            stream = connection.accept_bi() => {
                let stream = stream?;
                let fut = handle_stream_bidirectional(connection_handle.clone(), stream.0, stream.1);
                tokio::task::spawn(async move {
                    fut.await;
                });
            }
            stream = connection.accept_uni() => {
                let stream = stream?;
                let fut = handle_stream_unidirectional(connection_handle.clone(), stream);
                tokio::task::spawn(async move {
                    fut.await;
                });
            }
            Some(notification) = receiver.recv() => {
                match notification {
                    ConnectionNotification::ConnectionError(error) => {
                        println!("[HTTP/3] Received ConnectionError notification, closing connection {}: {error:?}", connection_handle.id);
                        connection.close(VarInt::from_u32(error as _), &[]);
                        return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "servente"));
                    }
                }
            }
        }
    }
}

async fn handle_stream_bidirectional(connection_handle: ConnectionHandle, send_stream: SendStream, recv_stream: RecvStream) {
    let mut recv_stream = recv_stream;
    let mut send_stream = send_stream;

    loop {
        let frame = match recv_stream.read_frame().await {
            Ok(frame) => frame,
            Err(e) => {
                if e.is_stream_closed() {
                    // The stream was closed, this is not a
                    // connection-broad error.
                    println!("[HTTP/3][{}/{}] Stream closed", connection_handle.id, recv_stream.id());
                    return;
                }

                println!("[HTTP/3][{}/{}] Read frame error: {e:?}", connection_handle.id, recv_stream.id());
                _ = connection_handle.notifier.send(
                    ConnectionNotification::ConnectionError(ErrorCode::H3FrameError)
                ).await;
                return;
            }
        };

        println!("[HTTP/3][{}/{}] Received frame: {frame:?}", connection_handle.id, recv_stream.id());

        match frame {
            _ => ()
        }
    }
}

async fn handle_stream_unidirectional(connection_handle: ConnectionHandle, recv_stream: RecvStream) {
    let mut recv_stream = recv_stream;

    let stream_type = recv_stream.read_variable_integer().await;
    if stream_type.is_err() {
        return;
    }

    let stream_type = stream_type.unwrap();
    let stream_type = match stream_type {
        0x00 => UnidirectionalStreamType::Control,
        0x01 => {
            _ = connection_handle.notifier.send(ConnectionNotification::ConnectionError(ErrorCode::H3StreamCreationError)).await;
            return;
        }
        0x02 => UnidirectionalStreamType::QpackEncoderStream,
        0x03 => UnidirectionalStreamType::QpackDecoderStream,
        _ => {
            println!("[HTTP/3] Client tried to start a unidirectional stream of unknown type: {stream_type}, ignoring");
            _ = recv_stream.stop(VarInt::from_u32(ErrorCode::H3StreamCreationError as _));
            return;
        }
    };

    println!("[HTTP/3] New unidirectional stream of type {stream_type:?} with id {}" , recv_stream.id());

    match stream_type {
        UnidirectionalStreamType::Control => handle_stream_unidirectional_control(connection_handle, recv_stream).await,
        UnidirectionalStreamType::Push(_) => {
            // Clients can't open _Server Push_ streams.
            // TODO is this error correct?
            _ = connection_handle.notifier.send(ConnectionNotification::ConnectionError(ErrorCode::H3StreamCreationError)).await;
        }
        UnidirectionalStreamType::QpackEncoderStream => {
            QpackEncoderReceiveStream::spawn(recv_stream, connection_handle).await;
        }
        UnidirectionalStreamType::QpackDecoderStream => {
            // TODO
        }
    }
}

async fn handle_stream_unidirectional_control(connection_handle: ConnectionHandle, mut recv_stream: RecvStream) {
    let mut has_sent_settings = false;

    loop {
        let frame = match recv_stream.read_frame().await {
            Ok(frame) => frame,
            Err(e) => {
                println!("[HTTP/3] Read frame error: {e:?}");
                if let ReadError::IoError(e) = e {
                    if e.kind() == io::ErrorKind::NotConnected {
                        return;
                    }
                }

                _ = connection_handle.notifier.send(
                    ConnectionNotification::ConnectionError(ErrorCode::H3FrameError)
                ).await;
                return;
            }
        };

        println!("[HTTP/3][{}/{}/ControlStream] Received frame: {frame:?}", connection_handle.id, recv_stream.id());

        match frame {
            Frame::Settings { settings } => {
                if has_sent_settings {
                    println!("[HTTP/3] Subsequent SETTINGS frame: {settings:?}");
                    _ = connection_handle.notifier.send(
                        ConnectionNotification::ConnectionError(ErrorCode::H3FrameUnexpected)
                    ).await;
                    return;
                }

                has_sent_settings = true;
            }
            _ => ()
        }
    }
}

pub async fn start(tls_config: Arc<rustls::ServerConfig>) -> io::Result<()> {
    let config = quinn::ServerConfig::with_crypto(tls_config);
    let endpoint = quinn::Endpoint::server(config, SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8080))
        .unwrap();

    while let Some(connection) = endpoint.accept().await {
        tokio::task::spawn(async move {
            let res = handle_connection(connection).await;
            if let Err(e) = res {
                println!("[HTTP/3] Connection error: {e:#?}");
            }
            _ = res;
        });
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PriorityUpdateOrigin {
    RequestStream,
    PushStream,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PushId(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum StreamOrPushId {
    StreamId(quinn::StreamId),
    PushId(PushId),
}

impl Into<usize> for StreamOrPushId {
    fn into(self) -> usize {
        match self {
            Self::PushId(id) => id.0,
            Self::StreamId(id) => id.0 as usize,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(usize)]
pub enum UnidirectionalStreamType {
    Control,
    Push(PushId),
    QpackEncoderStream,
    QpackDecoderStream,
}

impl Into<usize> for UnidirectionalStreamType {
    fn into(self) -> usize {
        match self {
            Self::Control => 0x00,
            Self::Push(_) => 0x01,
            Self::QpackEncoderStream => 0x02,
            Self::QpackDecoderStream => 0x03,
        }
    }
}
