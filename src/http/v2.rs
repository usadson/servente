// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use tokio::{
    io::{
        BufReader,
        BufWriter,
        ReadHalf,
        WriteHalf, AsyncReadExt, AsyncWriteExt,
    },
    net::TcpStream,
};

use tokio_rustls::server::TlsStream;

mod bits;

type Reader = BufReader<ReadHalf<TlsStream<TcpStream>>>;
type Writer = BufWriter<WriteHalf<TlsStream<TcpStream>>>;

/// I *would* use an enum for this in C-like languages, but Rust explicitly
/// considers illegal enum descriminants as undefined behavior, so I'll just use
/// these ugly constants instead.
///
/// [IANA: HTTP/2 Frame Types](https://www.iana.org/assignments/http2-parameters/http2-parameters.xhtml#frame-type)
const FRAME_TYPE_DATA: u8 = 0x00;
const FRAME_TYPE_HEADERS: u8 = 0x01;
const FRAME_TYPE_PRIORITY: u8 = 0x02;
const FRAME_TYPE_RST_STREAM: u8 = 0x03;
const FRAME_TYPE_SETTINGS: u8 = 0x04;
const FRAME_TYPE_PUSH_PROMISE: u8 = 0x05;
const FRAME_TYPE_PING: u8 = 0x06;
const FRAME_TYPE_GOAWAY: u8 = 0x07;
const FRAME_TYPE_WINDOW_UPDATE: u8 = 0x08;
const FRAME_TYPE_CONTINUATION: u8 = 0x09;
const FRAME_TYPE_ALTSVC: u8 = 0x0a;
const FRAME_TYPE_ORIGIN: u8 = 0x0c;


const MAXIMUM_ALLOWED_FRAME_SIZE: u32 = 0x00FF_FFFF;
const MAXIMUM_FLOW_CONTROL_WINDOW_SIZE: u32 = 0x7FFF_FFFF;


const SETTINGS_HEADER_TABLE_SIZE: u16 = 0x00_01;
const SETTINGS_ENABLE_PUSH: u16 = 0x00_02;
const SETTINGS_MAX_CONCURRENT_STREAMS: u16 = 0x00_03;
const SETTINGS_INITIAL_WINDOW_SIZE: u16 = 0x00_04;
const SETTINGS_MAX_FRAME_SIZE: u16 = 0x00_05;
const SETTINGS_MAX_HEADER_LIST_SIZE: u16 = 0x00_06;

const SETTINGS_ENABLE_CONNECT_PROTOCOL: u16 = 0x00_08;
const SETTINGS_NO_RFC7540_PRIORITIES: u16 = 0x00_09;

const SETTINGS_TLS_RENEG_PERMITTED: u16 = 0x00_10;

/// The `Connection` struct manages the state of the HTTP/2 connection.
struct Connection {
    reader: Reader,
    writer: Writer,
    settings: Settings,
}

impl Connection {
    pub fn new(reader: Reader, writer: Writer) -> Self {
        Self {
            reader,
            writer,
            settings: Settings::new(),
        }
    }

    /// Complete the connection preface by consuming the client settings,
    /// acknowledging them and sending our own settings.
    ///
    /// ### Note
    /// This does not include parsing the
    /// [connection preface string](https://www.rfc-editor.org/rfc/rfc9113.html#section-3.4-2),
    /// since this should already have been achieved by the HTTP/1.1 upgrade.
    ///
    /// ### References
    /// * [RFC 9113 - Section 3.4. HTTP/2 Connection Preface](https://www.rfc-editor.org/rfc/rfc9113.html#name-http-2-connection-preface)
    pub async fn complete_preface(&mut self) -> Result<(), ConnectionError> {
        self.send_frame_with_flush(Frame::Settings { settings: Vec::new() }).await?;

        let frame = self.read_frame().await?;
        let Frame::Settings { settings } = frame else {
            return Err(ConnectionError::ConnectionError {
                error_code: ErrorCode::ProtocolError,
                additional_debug_data: format!("Expected a SETTINGS frame to finish preface, but got a: {}", frame.frame_type())
            });
        };

        self.settings.apply(settings);

        Ok(())
    }

    pub async fn read_frame(&mut self) -> Result<Frame, ConnectionError> {
        let payload_length = self.read_payload_length().await?;
        let frame_type = self.reader.read_u8().await?;
        let flags = self.reader.read_u8().await?;
        let stream_id = StreamId(self.reader.read_u32().await? & 0x7FFF_FFFF);

        println!("[HTTP/2] [Frame] Received type {:x}, size {}, flags {:x} on stream {}", frame_type, payload_length, flags, stream_id.0);

        if payload_length > self.settings.maximum_payload_size.0 {
            return Err(ConnectionError::StreamError { error_code: ErrorCode::FrameSizeError, stream_id });
        }

        let mut payload = Vec::new();
        payload.resize(payload_length as _, 0);
        self.reader.read_exact(payload.as_mut_slice()).await?;

        match frame_type {
            FRAME_TYPE_PRIORITY => {
                if stream_id == StreamId::CONTROL {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: "PRIORITY on the CONTROL stream".to_string() })
                }

                if payload_length != 5 {
                    return Err(ConnectionError::StreamError { error_code: ErrorCode::FrameSizeError, stream_id });
                }

                return Ok(Frame::Unknown);
            }

            FRAME_TYPE_SETTINGS => {
                if stream_id != StreamId::CONTROL {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: "SETTINGS should be sent on frame 0".to_string() })
                }

                // ACK
                if flags & 0x01 == 1 {
                    if payload_length != 0 {
                        return Err(ConnectionError::ConnectionError { error_code: ErrorCode::FrameSizeError, additional_debug_data: "ACK'd SETTINGS should be 0 length".to_string() });
                    }
                    return Ok(Frame::SettingsAcknowledgement);
                }

                if payload_length == 0 {
                    return Ok(Frame::Settings { settings: Vec::new() });
                }

                if payload_length % 6 != 0 {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::FrameSizeError, additional_debug_data: "SETTINGS frame length should be a multiple of 6".to_string() })
                }

                let mut settings = Vec::with_capacity((payload_length / 6) as _);
                for data in payload.chunks_exact(6) {
                    let kind = u16::from_be_bytes([data[0], data[1]]);
                    let value = u32::from_be_bytes([data[2], data[3], data[4], data[5]]);
                    println!("[HTTP/2] [Settings] New setting of kind {} valued {}", kind, value);
                    settings.push(match kind {
                        SETTINGS_HEADER_TABLE_SIZE => (SettingKind::HeaderTableSize, SettingValue(value)),
                        SETTINGS_ENABLE_PUSH => {
                            if value != 0 && value != 1 {
                                return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: "ENABLE_PUSH invalid value: neither 0 nor 1".to_string() });
                            }
                            (SettingKind::EnablePush, SettingValue(value))
                        }
                        SETTINGS_MAX_CONCURRENT_STREAMS => (SettingKind::MaxConcurrentStreams, SettingValue(value)),
                        SETTINGS_INITIAL_WINDOW_SIZE => {
                            if value > MAXIMUM_FLOW_CONTROL_WINDOW_SIZE {
                                return Err(ConnectionError::ConnectionError { error_code: ErrorCode::FlowControlError, additional_debug_data: "Maximum flow-control window size exceeded".to_string() });
                            }
                            (SettingKind::InitialWindowSize, SettingValue(value))
                        }
                        SETTINGS_MAX_FRAME_SIZE => {
                            if value < SettingKind::MaxFrameSize.default_value().0 {
                                return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: "Maximum allowed frame size less than the initial frame size".into() });
                            }
                            if value > MAXIMUM_ALLOWED_FRAME_SIZE {
                                return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: "Maximum allowed frame size exceeded".into() });
                            }
                            (SettingKind::MaxFrameSize, SettingValue(value))
                        }
                        SETTINGS_MAX_HEADER_LIST_SIZE => (SettingKind::MaxHeaderListSize, SettingValue(value)),
                        _ => {
                            println!("[HTTP/2] [Settings] Received unknown setting of type {} with value {}", kind, value);
                            continue;
                        }
                    })
                }
                return Ok(Frame::Settings { settings });
            }

            /// https://www.rfc-editor.org/rfc/rfc9113.html#name-window_update
            FRAME_TYPE_WINDOW_UPDATE => {
                if payload_length != 4 {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::FrameSizeError, additional_debug_data: String::new() })
                }

                return Ok(Frame::Unknown);
            }

            _ => {
                // TODO: better skip the data without copying in userspace.
                Ok(Frame::Unknown)
            }
        }
    }

    /// ### References
    /// * [RFC 9113: Section 4.2. Frame Size](https://www.rfc-editor.org/rfc/rfc9113.html#name-frame-size)
    async fn read_payload_length(&mut self) -> Result<u32, ConnectionError> {
        let mut buf: [u8; 3] = [0; 3];
        self.reader.read_exact(&mut buf).await?;
        Ok(bits::convert_be_u24_to_u32(buf))
    }

    pub async fn send_frame(&mut self, frame: Frame) -> Result<(), ConnectionError> {
        let flags = frame.flags();
        let frame_type = frame.frame_type();
        let stream = frame.stream();

        let payload = frame.into_payload();

        self.writer.write_all(&(payload.len() as u32).to_be_bytes()[0..3]).await?;
        self.writer.write_u8(frame_type).await?;
        self.writer.write_u8(flags).await?;
        self.writer.write_u32(stream.0 & 0x7FFF_FFFF).await?;
        self.writer.write_all(payload.as_slice()).await?;
        Ok(())
    }

    pub async fn send_frame_with_flush(&mut self, frame: Frame) -> Result<(), ConnectionError> {
        self.send_frame(frame).await?;
        self.writer.flush().await?;
        Ok(())
    }
}

#[derive(Debug)]
enum ConnectionError {
    Io(std::io::Error),
    ConnectionError {
        error_code: ErrorCode,
        additional_debug_data: String,
    },
    StreamError{
        error_code: ErrorCode,
        stream_id: StreamId,
    },
}

impl From<std::io::Error> for ConnectionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum ErrorCode {
    NoError = 0,
    ProtocolError = 1,
    InternalError = 2,
    FlowControlError = 3,
    SettingsTimeout = 4,
    StreamClosed = 5,
    FrameSizeError = 6,
    RefusedStream = 7,
    Cancel = 8,
    CompressionError = 9,
    ConnectError = 10,
    EnhanceYourCalm = 11,
    InadequateSecurity = 12,
    Http11Required = 13,
}

/// The unit of communication in an HTTP/2 connection.
#[derive(Debug)]
enum Frame {
    Data {
        end_stream: bool,
        stream: StreamId,
        payload: Vec<u8>,
    },
    GoAway {
        last_stream_id: StreamId,
        error_code: ErrorCode,
        additional_debug_data: String,
    },
    /// https://www.rfc-editor.org/rfc/rfc9113.html#name-rst_stream
    ResetStream {
        stream_id: StreamId,
        error_code: ErrorCode,
    },
    Settings {
        settings: Vec<(SettingKind, SettingValue)>,
    },
    SettingsAcknowledgement,

    /// Unknown frames of some type, MUST be ignored and is discarded.
    Unknown,
}

impl Frame {
    /// Generate the FLAGS for this frame.
    const fn flags(&self) -> u8 {
        match self {
            Frame::Data { end_stream, .. } if *end_stream => 0b0000_0001,
            Frame::Data { .. } => 0,
            Frame::GoAway { .. } => 0,
            Frame::ResetStream { .. } => 0,
            Frame::Settings { .. } => 0,
            Frame::SettingsAcknowledgement => 0b0000_0001,
            Frame::Unknown => unreachable!(),
        }
    }

    const fn frame_type(&self) -> u8 {
        match self {
            Frame::Data { .. } => FRAME_TYPE_DATA,
            Frame::GoAway { .. } => FRAME_TYPE_GOAWAY,
            Frame::ResetStream { .. } => FRAME_TYPE_RST_STREAM,
            Frame::Settings { .. } => FRAME_TYPE_SETTINGS,
            Frame::SettingsAcknowledgement => FRAME_TYPE_SETTINGS,
            Frame::Unknown => unreachable!(),
        }
    }

    fn into_payload(self) -> Vec<u8> {
        match self {
            Frame::Data { payload, .. } => payload,
            Frame::GoAway { last_stream_id, error_code, additional_debug_data } => {
                let mut payload = Vec::with_capacity(4 + 4 + additional_debug_data.len());
                payload.extend_from_slice(&(last_stream_id.0 & 0x7FFF_FFFF).to_be_bytes());
                payload.extend_from_slice(&(error_code as u32).to_be_bytes());
                payload.extend_from_slice(additional_debug_data.as_bytes());
                payload
            }
            Frame::ResetStream { error_code, .. } => {
                Vec::from((error_code as u32).to_be_bytes())
            }
            Frame::Settings { settings } => {
                let mut payload = Vec::with_capacity(settings.len() * 6);
                for (kind, value) in settings {
                    payload.extend((kind as u16).to_be_bytes());
                    payload.extend(value.0.to_be_bytes());
                }
                payload
            }
            Frame::SettingsAcknowledgement => Vec::new(),
            Frame::Unknown => unreachable!(),
        }
    }

    const fn stream(&self) -> StreamId {
        match self {
            Frame::Data { stream, .. } => *stream,
            Frame::GoAway { .. } => StreamId::CONTROL,
            Frame::ResetStream { stream_id, .. } => *stream_id,
            Frame::Settings { .. } => StreamId::CONTROL,
            Frame::SettingsAcknowledgement => StreamId::CONTROL,
            Frame::Unknown => unreachable!(),
        }
    }
}

/// The generic frame header, before specialisation in the [`Frame`] enum.
///
/// There exist some special crates for handling bitfields like in C, but they
/// are quite awkward to use, so this struct just upgrades them to this Rust
/// format.
#[derive(Debug)]
struct FrameHeader {
    // Max is 24
    length: u32,
    frame_type: u8,
    flags: u8,
    stream_identifier: u32,
}

#[derive(Debug)]
#[repr(u16)]
enum SettingKind {
    HeaderTableSize = 0x01,
    EnablePush = 0x02,
    MaxConcurrentStreams = 0x03,
    InitialWindowSize = 0x04,
    MaxFrameSize = 0x05,
    MaxHeaderListSize = 0x06,

    /// Enable the CONNECT protocol.
    ///
    /// ### References
    /// * [RFC 8441](https://www.iana.org/go/rfc8441)
    SettingsEnableConnectProtocol = 0x08,

    /// Disable the priorities system
    ///
    /// ### References
    /// * [RFC 9218](https://www.iana.org/go/rfc9218)
    SettingsNoRfc7540Priorities = 0x09,

    /// ### References
    /// * [MS-HTTP2E: Hypertext Transfer Protocol Version 2 (HTTP/2) Extension](https://winprotocoldoc.blob.core.windows.net/productionwindowsarchives/MS-HTTP2E/%5bMS-HTTP2E%5d.pdf)
    TlsRenegotiationPermitted = 0x10,
}

impl SettingKind {
    pub const fn default_value(&self) -> SettingValue {
        SettingValue(match self {
            SettingKind::HeaderTableSize => 4096,
            SettingKind::EnablePush => 1,
            SettingKind::MaxConcurrentStreams => u32::MAX,
            SettingKind::InitialWindowSize => 65535,
            SettingKind::MaxFrameSize => 16384,
            SettingKind::MaxHeaderListSize => u32::MAX,
            SettingKind::SettingsEnableConnectProtocol => 0,
            SettingKind::SettingsNoRfc7540Priorities => 0,
            SettingKind::TlsRenegotiationPermitted => 0,
        })
    }
}

#[derive(Debug)]
struct Settings {
    maximum_payload_size: SettingValue,
}

impl Settings {
    pub const fn new() -> Self {
        Self {
            maximum_payload_size: SettingKind::MaxFrameSize.default_value(),
        }
    }

    fn apply(&mut self, settings: Vec<(SettingKind, SettingValue)>) {
        for (kind, value) in settings {
            match kind {
                SettingKind::MaxFrameSize => self.maximum_payload_size = value,
                _ => ()
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SettingValue(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamId(pub u32);

impl StreamId {
    pub const CONTROL: StreamId = StreamId(0);
}

/// Entrypoint of the client connection.
///
/// Returning from this function means the connection should/has been closed.
///
/// ### HTTP/1.1
/// When upgraded from HTTP/1.1, this is after the PRI preface stuff, but no
/// frames are read yet.
pub async fn handle_client(reader: Reader, writer: Writer) {
    let mut connection = Connection::new(reader, writer);

    if let Err(e) = connection.complete_preface().await {
        #[cfg(feature = "debugging")]
        println!("[HTTP/2] [Preface] Failed to complete preface: {:#?}", e);
        match e {
            ConnectionError::ConnectionError { error_code, additional_debug_data } => {
                _ = connection.send_frame(Frame::GoAway {
                    last_stream_id: StreamId::CONTROL,
                    error_code,
                    additional_debug_data,
                }).await;
            }
            ConnectionError::StreamError { error_code, .. } => {
                _ = connection.send_frame(Frame::GoAway {
                    last_stream_id: StreamId::CONTROL,
                    error_code,
                    additional_debug_data: "Stream Error on preface completion".to_owned(),
                }).await;
            }
            ConnectionError::Io(_) => (),
        };
        return;
    }

    loop {
        if let Err(e) = handle_client_inner(&mut connection).await {
            #[cfg(feature = "debugging")]
            println!("[HTTP/2] Handle client error: {:#?}", e);
            match e {
                ConnectionError::ConnectionError { error_code, additional_debug_data } => {
                    _ = connection.send_frame(Frame::GoAway {
                        last_stream_id: StreamId::CONTROL,
                        error_code,
                        additional_debug_data,
                    }).await;
                    return;
                }
                ConnectionError::StreamError { error_code, stream_id } => {
                    if connection.send_frame(Frame::ResetStream {
                        stream_id,
                        error_code,
                    }).await.is_err() {
                        return;
                    }
                    // TODO mark stream as "closed"
                }
                ConnectionError::Io(_) => return,
            };
        }
    }
}

async fn handle_client_inner(connection: &mut Connection) -> Result<(), ConnectionError> {
    loop {
        let frame = connection.read_frame().await?;
        println!("[HTTP/2] Incoming frame: {:#?}", frame);
    }
}
