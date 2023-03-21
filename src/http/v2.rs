// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::sync::Arc;

use tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
        BufReader,
        BufWriter,
        ReadHalf,
        WriteHalf,
    },
    net::TcpStream,
    sync::Mutex,
    task::JoinHandle,
};

use tokio_rustls::server::TlsStream;

use crate::ServenteConfig;

use self::hpack::DynamicTable;

use super::message::{Response, Request};

mod bits;
mod hpack;

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

struct BinaryRequest {
    /// The stream from where the request was initiated from and where the
    /// response should be sent to.
    stream_id: StreamId,

    /// A list of all the header data, first should be from the HEADERS frame,
    /// optionally the others from CONTINUATION frames.
    headers: Vec<Vec<u8>>,

    data: Vec<Vec<u8>>,

    /// The byte position in the header stream.
    cursor: usize,
}

impl BinaryRequest {
    #[inline]
    pub async fn decode(self, dynamic_table: Arc<Mutex<DynamicTable>>) -> Result<Request, RequestError> {
        hpack::decode_hpack(self, dynamic_table).await.map_err(|e| RequestError::CompressionError(e))
    }

    /// Get the length of all the data inside the headers.
    pub fn len(&self) -> usize {
        self.headers.iter().map(|v| v.len()).sum()
    }

    pub fn peek_u8(&self) -> Option<u8> {
        let mut cursor = self.cursor;
        for vec in &self.headers {
            if vec.len() > cursor {
                return Some(vec[cursor]);
            }

            cursor -= vec.len();
        }

        None
    }

    pub fn read_integer(&mut self, first_byte: u8, n: u32) -> Option<usize> {
        let mut i = first_byte as usize;
        if i < (2_usize.pow(n) - 1) {
            return Some(i);
        }

        let mut m = 0;
        while let Some(octet) = self.read_u8() {
            i += ((octet & 0x7F) as usize) * 2_usize.pow(m);
            m += 7;

            if octet & 0x80 != 0x80 {
                return Some(i);
            }
        }

        None
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        if let Some(val) = self.peek_u8() {
            self.cursor += 1;
            Some(val)
        } else {
            None
        }
    }

    fn read_string(&mut self) -> Option<String> {
        let Some(first_octet) = self.read_u8() else {
            return None;
        };

        let is_huffman = first_octet & 0x80 == 0x80;
        let Some(length) = self.read_integer(first_octet & 0x7F, 7) else {
            return None
        };

        let mut vec = Vec::new();
        for i in 0..length {
            let Some(byte) = self.read_u8() else {
                return None;
            };

            vec.push(byte);
        }

        if !is_huffman {
            return String::from_utf8(vec).ok();
        }

        hpack::decode_huffman(vec.as_slice())
    }
}

struct ConcurrentContext {
    servente_config: Arc<ServenteConfig>,
    dynamic_table: Arc<Mutex<DynamicTable>>,
    receiver: tokio::sync::mpsc::Receiver<(StreamId, Result<Response, RequestError>)>,
    sender: tokio::sync::mpsc::Sender<(StreamId, Result<Response, RequestError>)>,
    requests: hashbrown::HashMap<StreamId, JoinHandle<()>>,
}

impl ConcurrentContext {
    pub fn new(servente_config: Arc<ServenteConfig>) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        Self {
            servente_config,
            dynamic_table: Arc::new(Mutex::new(DynamicTable::new(SettingKind::HeaderTableSize.default_value().0 as _))),
            receiver,
            sender,
            requests: Default::default(),
        }
    }
}

impl Drop for ConcurrentContext {
    fn drop(&mut self) {
        for join_handle in self.requests.values() {
            join_handle.abort();
        }
    }
}

/// The `Connection` struct manages the state of the HTTP/2 connection.
struct Connection {
    servente_config: Arc<ServenteConfig>,
    reader: Reader,
    writer: Writer,
    settings: Settings,
    continuation: Option<StreamId>,
    streams: hashbrown::HashMap<StreamId, Stream>,
    header_compressor: hpack::Compressor,
}

impl Connection {
    pub fn new(reader: Reader, writer: Writer, servente_config: Arc<ServenteConfig>) -> Self {
        Self {
            servente_config,
            reader,
            writer,
            settings: Settings::new(),
            continuation: None,
            streams: Default::default(),
            header_compressor: hpack::Compressor::new(),
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

        self.send_frame_with_flush(Frame::SettingsAcknowledgement).await?;
        Ok(())
    }

    pub async fn read_frame(&mut self) -> Result<Frame, ConnectionError> {
        let payload_length = self.read_payload_length().await?;
        let frame_type = self.reader.read_u8().await?;
        let flags = self.reader.read_u8().await?;
        let stream_id = StreamId(self.reader.read_u32().await? & 0x7FFF_FFFF);

        #[cfg(feature = "debugging")]
        println!("[HTTP/2] [Frame] Received type {:x}, size {}, flags {:x} on stream {}", frame_type, payload_length, flags, stream_id.0);

        if payload_length > self.settings.maximum_payload_size.0 {
            return Err(ConnectionError::StreamError { error_code: ErrorCode::FrameSizeError, stream_id });
        }

        let mut payload = Vec::new();
        payload.resize(payload_length as _, 0);
        self.reader.read_exact(payload.as_mut_slice()).await?;

        // https://www.rfc-editor.org/rfc/rfc9113.html#section-6.2-6.6.2
        if let Some(continuation_stream) = self.continuation {
            if continuation_stream != stream_id || frame_type != FRAME_TYPE_CONTINUATION {
                return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: "CONTINUATION expected".to_string() });
            }
        }

        match frame_type {
            FRAME_TYPE_DATA => {
                // https://www.rfc-editor.org/rfc/rfc9113.html#section-6.2-8
                if stream_id == StreamId::CONTROL {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: "DATA on the CONTROL stream".to_string() });
                }

                if payload_length == 0 {
                    return Err(ConnectionError::StreamError { error_code: ErrorCode::FrameSizeError, stream_id });
                }

                let is_padded = flags & 0x08 == 0x08;
                let end_stream = flags & 0x01 == 0x01;

                let data_start = is_padded.then_some(1).unwrap_or(0);
                if payload_length < data_start {
                    return Err(ConnectionError::StreamError { error_code: ErrorCode::FrameSizeError, stream_id });
                }

                let data_end = (payload_length - is_padded.then_some(payload[0] as u32).unwrap_or(0)) as usize;
                let data_start = data_start as usize;

                Ok(Frame::Data {
                    end_stream,
                    stream_id,
                    payload: Vec::from(&payload[data_start..data_end])
                })
            }

            // https://www.rfc-editor.org/rfc/rfc9113.html#name-headers
            FRAME_TYPE_HEADERS => {
                // https://www.rfc-editor.org/rfc/rfc9113.html#section-6.2-8
                if stream_id == StreamId::CONTROL {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: "HEADERS on the CONTROL stream".to_string() });
                }

                if payload_length == 0 {
                    return Err(ConnectionError::StreamError { error_code: ErrorCode::FrameSizeError, stream_id });
                }

                let is_padded = flags & 0x08 == 0x08;
                let is_priority = flags & 0x20 == 0x20;
                let end_headers = flags & 0x04 == 0x04;
                let end_stream = flags & 0x01 == 0x01;

                if !end_headers {
                    self.continuation = Some(stream_id);
                }

                let padding = is_padded.then_some(1).unwrap_or(0);
                let data_start = padding + is_priority.then_some(5).unwrap_or(0);
                if payload_length < data_start {
                    return Err(ConnectionError::StreamError { error_code: ErrorCode::FrameSizeError, stream_id });
                }

                let data_end = (payload_length - is_padded.then_some(payload[0] as u32).unwrap_or(0)) as usize;
                let data_start = data_start as usize;

                Ok(Frame::Headers {
                    end_headers,
                    end_stream,
                    stream_id,
                    payload: Vec::from(&payload[data_start..data_end])
                })
            }

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
                            #[cfg(feature = "debugging")]
                            println!("[HTTP/2] [Settings] Received unknown setting of type {} with value {}", kind, value);
                            continue;
                        }
                    })
                }
                return Ok(Frame::Settings { settings });
            }

            FRAME_TYPE_GOAWAY => {
                if payload_length < 8 {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::FrameSizeError, additional_debug_data: String::from("Illegal GOAWAY size") });
                }
                if stream_id != StreamId::CONTROL {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: String::from("GOAWAY on non-control stream") });
                }
                Ok(Frame::GoAway {
                    last_stream_id: StreamId(u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) & 0x7F),
                    error_code: ErrorCode::from(u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]])),
                    additional_debug_data: String::from_utf8_lossy(&payload[8..]).to_string(),
                })
            }

            // https://www.rfc-editor.org/rfc/rfc9113.html#name-window_update
            FRAME_TYPE_WINDOW_UPDATE => {
                if payload_length != 4 {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::FrameSizeError, additional_debug_data: String::new() })
                }

                Ok(Frame::Unknown)
            }

            FRAME_TYPE_CONTINUATION => {
                if !self.continuation.is_some() {
                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: String::from("CONTINUATION frame without corresponding HEADERS") });
                }

                let end_headers = flags & 0x04 == 0x04;
                Ok(Frame::Continuation { end_headers, stream_id, payload })
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

    async fn send_data_frame_from_slice(&mut self, stream_id: StreamId, data: &[u8]) -> Result<(), ConnectionError> {
        let mut sent = 0;
        for chunk in data.chunks(self.settings.maximum_payload_size.0 as _) {
            sent += chunk.len();
            let end_stream = sent == data.len();

            self.send_frame(Frame::Data {
                end_stream, stream_id, payload: Vec::from(chunk)
            }).await?;
        }

        Ok(())
    }

    pub async fn send_frame(&mut self, frame: Frame) -> Result<(), ConnectionError> {
        send_frame(&mut self.writer, frame).await
    }

    pub async fn send_frame_with_flush(&mut self, frame: Frame) -> Result<(), ConnectionError> {
        self.send_frame(frame).await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn send_response(&mut self, stream_id: StreamId, response: Response) -> Result<(), ConnectionError> {
        let payload = self.header_compressor.compress(&response);
        self.send_frame(Frame::Headers { end_headers: true, end_stream: response.body.is_none(), stream_id, payload }).await?;

        if let Some(body) = response.body {
            match body {
                super::message::BodyKind::Bytes(data) => {
                    self.send_data_frame_from_slice(stream_id, &data).await?;
                }
                super::message::BodyKind::CachedBytes(versions, coding) => {
                    self.send_data_frame_from_slice(stream_id, versions.get_version(coding)).await?;
                }
                super::message::BodyKind::File(file) => todo!(),
                super::message::BodyKind::String(data) => {
                    self.send_data_frame_from_slice(stream_id, data.as_bytes()).await?;
                }
                super::message::BodyKind::StaticString(str) => {
                    self.send_data_frame_from_slice(stream_id, str.as_bytes()).await?;
                }
            }
        }

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

impl From<u32> for ErrorCode {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::NoError,
            1 => Self::ProtocolError,
            2 => Self::InternalError,
            3 => Self::FlowControlError,
            4 => Self::SettingsTimeout,
            5 => Self::StreamClosed,
            6 => Self::FrameSizeError,
            7 => Self::RefusedStream,
            8 => Self::Cancel,
            9 => Self::CompressionError,
            10 => Self::ConnectError,
            11 => Self::EnhanceYourCalm,
            12 => Self::InadequateSecurity,
            13 => Self::Http11Required,
            _ => {
                #[cfg(feature = "debugging")]
                println!("[HTTP/2] [Warning] Unknown error code: {value}");
                // [RFC 9113, section 7](https://httpwg.org/specs/rfc9113.html#rfc.section.7.p.5):
                // > Unknown or unsupported error codes MUST NOT trigger any
                // > special behavior. These MAY be treated by an implementation
                // > as being equivalent to INTERNAL_ERROR.
                Self::InternalError
            }
        }
    }
}

/// The unit of communication in an HTTP/2 connection.
#[derive(Debug)]
enum Frame {
    Data {
        end_stream: bool,
        stream_id: StreamId,
        payload: Vec<u8>,
    },
    Headers {
        end_headers: bool,
        end_stream: bool,
        stream_id: StreamId,
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

    Continuation {
        end_headers: bool,
        stream_id: StreamId,
        payload: Vec<u8>,
    },

    /// Unknown frames of some type, MUST be ignored and is discarded.
    Unknown,
}

impl Frame {
    /// Generate the FLAGS for this frame.
    fn flags(&self) -> u8 {
        match self {
            Frame::Data { end_stream, .. } if *end_stream => 0b0000_0001,
            Frame::Data { .. } => 0,
            Frame::GoAway { .. } => 0,
            Frame::Headers { end_headers, end_stream, .. } => {
                end_headers.then_some(0x04).unwrap_or(0) | end_stream.then_some(0x01).unwrap_or(0)
            }
            Frame::ResetStream { .. } => 0,
            Frame::Settings { .. } => 0,
            Frame::SettingsAcknowledgement => 0b0000_0001,
            Frame::Continuation { end_headers, .. } if *end_headers => 0b0000_0100,
            Frame::Continuation { .. } => 0,
            Frame::Unknown => unreachable!(),
        }
    }

    const fn frame_type(&self) -> u8 {
        match self {
            Frame::Data { .. } => FRAME_TYPE_DATA,
            Frame::GoAway { .. } => FRAME_TYPE_GOAWAY,
            Frame::Headers { .. } => FRAME_TYPE_HEADERS,
            Frame::ResetStream { .. } => FRAME_TYPE_RST_STREAM,
            Frame::Settings { .. } => FRAME_TYPE_SETTINGS,
            Frame::SettingsAcknowledgement => FRAME_TYPE_SETTINGS,
            Frame::Continuation { .. } => FRAME_TYPE_CONTINUATION,
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
            Frame::Headers { payload, .. } => payload,
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
            Frame::Continuation { payload, .. } => payload,
            Frame::SettingsAcknowledgement => Vec::new(),
            Frame::Unknown => unreachable!(),
        }
    }

    const fn stream(&self) -> StreamId {
        match self {
            Frame::Data { stream_id, .. } => *stream_id,
            Frame::GoAway { .. } => StreamId::CONTROL,
            Frame::Headers { stream_id, .. } => *stream_id,
            Frame::ResetStream { stream_id, .. } => *stream_id,
            Frame::Settings { .. } => StreamId::CONTROL,
            Frame::SettingsAcknowledgement => StreamId::CONTROL,
            Frame::Continuation { stream_id, .. } => *stream_id,
            Frame::Unknown => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum RequestError {
    CompressionError(hpack::DecompressionError),
}

async fn send_frame<T>(writer: &mut T, frame: Frame) -> Result<(), ConnectionError>
        where T: AsyncWriteExt + Unpin {
    let flags = frame.flags();
    let frame_type = frame.frame_type();
    let stream = frame.stream();


    let payload = frame.into_payload();

    #[cfg(feature = "debugging")]
    println!("[HTTP/2] Sending frame: type={frame_type:#x} flags={flags:#b}/{flags:#x} stream={stream:?} payload: {} bytes", payload.len());

    writer.write_all(&(payload.len() as u32).to_be_bytes()[1..4]).await?;
    writer.write_u8(frame_type).await?;
    writer.write_u8(flags).await?;
    writer.write_u32(stream.0 & 0x7FFF_FFFF).await?;
    writer.write_all(payload.as_slice()).await?;
    Ok(())
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

#[derive(Debug)]
struct Stream {
    state: StreamState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamId(pub u32);

#[derive(Debug)]
enum StreamState {
    Idle,
    ReservedLocal,
    ReservedRemote,
    Open,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
}

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
pub async fn handle_client(reader: Reader, writer: Writer, servente_config: Arc<ServenteConfig>) {
    let mut connection = Connection::new(reader, writer, servente_config);

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

    let mut concurrent_context = ConcurrentContext::new(Arc::clone(&connection.servente_config));

    loop {
        if let Err(e) = handle_client_inner(&mut connection, &mut concurrent_context).await {
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

async fn handle_client_inner(connection: &mut Connection, concurrent_context: &mut ConcurrentContext) -> Result<(), ConnectionError> {
    loop {
        tokio::select! {
            frame = connection.read_frame() => {
                let frame = frame?;
                #[cfg(feature = "debugging")]
                println!("[HTTP/2] Incoming frame: {:#?}", frame);

                match frame {
                    Frame::Headers { end_stream, payload, stream_id, .. } => {
                        let mut binary_request = BinaryRequest { stream_id, headers: vec![payload], data: Vec::new(), cursor: 0 };
                        while connection.continuation.is_some() {
                            let frame = connection.read_frame().await?;
                            debug_assert!(frame.frame_type() == FRAME_TYPE_CONTINUATION);
                            if let Frame::Continuation { payload, .. } = frame {
                                binary_request.headers.push(payload);
                            }
                        }

                        // TODO some other frames may come in the middle
                        if !end_stream {
                            loop {
                                let frame = connection.read_frame().await?;
                                if let Frame::Data { end_stream, stream_id, payload } = frame {
                                    if stream_id != binary_request.stream_id {
                                        return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: String::from("TODO: expected DATA frame on this stream") });
                                    }

                                    binary_request.data.push(payload);

                                    if end_stream {
                                        break;
                                    }
                                } else if let Frame::Headers { .. } = frame {
                                    // Trailers Frame
                                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: String::from("TODO: support HEADERS") });
                                } else {
                                    return Err(ConnectionError::ConnectionError { error_code: ErrorCode::ProtocolError, additional_debug_data: format!("TODO: expected DATA frame, got: {:#?}", frame) });
                                }
                            }
                        }

                        connection.streams.insert(stream_id, Stream {
                            state: StreamState::HalfClosedRemote,
                        });
                        concurrent_context.requests.insert(stream_id, tokio::spawn(handle_request(binary_request, concurrent_context.sender.clone(), Arc::clone(&concurrent_context.dynamic_table), Arc::clone(&concurrent_context.servente_config))));

                    }

                    _ => (),
                }
            }
            Some(result) = concurrent_context.receiver.recv() => handle_client_inner_join(connection, result, concurrent_context).await?,
        }
    }
}

async fn handle_client_inner_join(connection: &mut Connection, result: (StreamId, Result<Response, RequestError>), concurrent_context: &mut ConcurrentContext) -> Result<(), ConnectionError> {
    let (stream_id, response_result) = result;

    if let Some(join_handle) = concurrent_context.requests.remove(&stream_id) {
        join_handle.await.unwrap();
    }

    match response_result {
        Ok(response) => {
            connection.send_response(stream_id, response).await
        }
        Err(e) => match e {
            RequestError::CompressionError(error) => {
                Err(ConnectionError::ConnectionError { error_code: ErrorCode::CompressionError, additional_debug_data: format!("Stream {} failed to decompress: {:#?}", stream_id.0, error) })
            }
        }
    }
}

async fn handle_request(binary_request: BinaryRequest, sender: tokio::sync::mpsc::Sender<(StreamId, Result<Response, RequestError>)>,
        dynamic_table: Arc<Mutex<DynamicTable>>, config: Arc<ServenteConfig>) {
    let stream_id = binary_request.stream_id;
    let result = handle_request_inner(binary_request, dynamic_table, config).await;
    _ = sender.send((stream_id, result)).await;
}

async fn handle_request_inner(binary_request: BinaryRequest, dynamic_table: Arc<Mutex<DynamicTable>>, config: Arc<ServenteConfig>) -> Result<Response, RequestError> {
    let request = binary_request.decode(dynamic_table).await?;
    let mut response = super::handle_request(&request, config.as_ref()).await;
    super::finish_response_normal(&request, &mut response).await;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_frame() {
        let frame = Frame::Headers { end_headers: true, end_stream: true, stream_id: StreamId(1), payload: vec![0xDE] };
        let mut buf = Vec::new();
        send_frame(&mut buf, frame).await.unwrap();
        println!("Buf: {:#x?}", buf);

        println!("{:#x?}", &(1 as u32).to_le_bytes());
        assert_eq!(buf.len(), 10);

        assert_eq!(buf[0..3], [0x00, 0x00, 0x01], "Length incorrect");
        assert_eq!(buf[3], 0x01, "Type incorrect");
        assert_eq!(buf[4], 0b0000_0101, "Flags incorrect");
        assert_eq!(buf[5..9], [0x00, 0x00, 0x00, 0x01], "Stream ID incorrect");
        assert_eq!(buf[9], 0xDE, "Incorrect payload");
    }
}
