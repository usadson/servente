// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::io;

use async_trait::async_trait;
use hashbrown::HashMap;

use quinn::StreamId;

use tokio::io::{
    AsyncRead,
    AsyncReadExt,
    AsyncWriteExt,
};

use crate::{
    error::ErrorCode,
    Frame,
    PriorityUpdateOrigin,
    PushId,
    ReadError,
    StreamOrPushId, UnidirectionalStreamType,
};

/// Frame type of [`Frame::Data`].
pub const FRAME_TYPE_DATA: usize = 0x00;

/// Frame type of [`Frame::Headers`].
pub const FRAME_TYPE_HEADERS: usize = 0x01;

/// Frame type of [`Frame::CancelPush`].
pub const FRAME_TYPE_CANCEL_PUSH: usize = 0x03;

/// Frame type of [`Frame::Settings`].
pub const FRAME_TYPE_SETTINGS: usize = 0x04;

/// Frame type of [`Frame::PushPromise`].
pub const FRAME_TYPE_PUSH_PROMISE: usize = 0x05;

/// Frame type of [`Frame::GoAway`].
pub const FRAME_TYPE_GOAWAY: usize = 0x07;

/// Frame type of [`Frame::Origin`].
pub const FRAME_TYPE_ORIGIN: usize = 0x0c;

/// Frame type of [`Frame::MaxPushId`].
pub const FRAME_TYPE_MAX_PUSH_ID: usize = 0x0d;

/// Frame type of [`Frame::Metadata`].
pub const FRAME_TYPE_METADATA: usize = 0x4d;

/// Frame type of [`Frame::PriorityUpdate`].
pub const FRAME_TYPE_PRIORITY_UPDATE_REQUEST: usize = 0xf0700;

/// Frame type of [`Frame::PriorityUpdate`].
pub const FRAME_TYPE_PRIORITY_UPDATE_PUSH: usize = 0xf0701;

#[derive(Debug)]
pub enum IoError {
    ConnectionError(quinn::ConnectionError),
    InvalidVariableLengthInteger,

    /// Invalid huffman code sent
    QpackHuffmanError,

    /// Invalid UTF-8 sent.
    QpackUtf8Error,

    StdIoError(io::Error),
}

impl From<quinn::ConnectionError> for IoError {
    fn from(value: quinn::ConnectionError) -> Self {
        Self::ConnectionError(value)
    }
}

impl From<io::Error> for IoError {
    fn from(value: io::Error) -> Self {
        Self::StdIoError(value)
    }
}

#[async_trait]
pub trait ReadExtensions: AsyncRead {
    async fn read_frame(&mut self) -> Result<Frame, ReadError>;

    async fn read_payload(&mut self, length: usize) -> Result<Vec<u8>, ReadError>;

    async fn read_qpack_integer(&mut self, first_byte: u8, n: u32) -> Result<usize, IoError>;

    /// Reading variable-length integers in HTTP/3 is the same as for QUIC.
    ///
    /// # References
    /// * [RFC 9000 - Section 16](https://www.rfc-editor.org/rfc/rfc9000.html#section-16)
    async fn read_variable_integer(&mut self) -> Result<usize, ReadError>;
}

#[async_trait]
impl<T> ReadExtensions for T
        where T: AsyncReadExt + Unpin + Send {
    async fn read_frame(&mut self) -> Result<Frame, ReadError> {
        let frame_type = self.read_variable_integer().await?;
        dbg!(frame_type);

        let length = self.read_variable_integer().await?;

        println!("read_frame type={frame_type} length={length}");

        Ok(match frame_type {
            FRAME_TYPE_DATA => Frame::Data { payload: self.read_payload(length).await? },
            FRAME_TYPE_HEADERS => Frame::Headers { payload: self.read_payload(length).await? },
            FRAME_TYPE_CANCEL_PUSH => Frame::CancelPush { push_id: PushId(self.read_variable_integer().await?) },
            FRAME_TYPE_SETTINGS => {
                let mut settings = HashMap::new();
                let payload = self.read_payload(length).await?;
                let mut payload = io::Cursor::new(payload);

                while (payload.position() as usize) < length {
                    let key = payload.read_variable_integer().await?;
                    let value = payload.read_variable_integer().await?;
                    if let Ok(key) = key.try_into() {
                        settings.insert(key, value);
                    }
                }

                Frame::Settings { settings }
            }
            FRAME_TYPE_PUSH_PROMISE => return Err(ReadError::ConnectionError(ErrorCode::H3FrameUnexpected)),
            FRAME_TYPE_GOAWAY => Frame::GoAway {
                stream_or_push_id: StreamOrPushId::PushId(PushId(self.read_variable_integer().await?))
            },
            FRAME_TYPE_ORIGIN => {
                let mut entries = Vec::new();
                let payload = self.read_payload(length).await?;
                let mut payload = io::Cursor::new(payload);

                while (payload.position() as usize) < length {
                    let len = payload.read_u16().await?;

                    let mut origin_as_bytes = Vec::new();
                    origin_as_bytes.resize(len as usize, 0);
                    _ = payload.read_exact(origin_as_bytes.as_mut_slice()).await?;

                    let origin_as_string = String::from_utf8(origin_as_bytes)
                        .map_err(|_| ReadError::NonAsciiOrigin)?;

                    entries.push(origin_as_string);
                }

                Frame::Origin { entries }
            },
            FRAME_TYPE_MAX_PUSH_ID => Frame::MaxPushId { push_id: PushId(self.read_variable_integer().await?) },
            FRAME_TYPE_METADATA => {
                _ = self.read_payload(length).await?;
                Frame::Metadata {  }
            }
            FRAME_TYPE_PRIORITY_UPDATE_REQUEST => {
                let id = self.read_variable_integer().await?;
                let value = self.read_payload(length).await?;
                let value = String::from_utf8(value)
                    .map_err(|_| ReadError::NonAsciiPriorityUpdate)?;

                Frame::PriorityUpdate {
                    origin: PriorityUpdateOrigin::RequestStream,
                    stream_or_push_id: StreamOrPushId::StreamId(StreamId(id as u64)),
                    value
                }
            }
            FRAME_TYPE_PRIORITY_UPDATE_PUSH => {
                let id = self.read_variable_integer().await?;
                let value = self.read_payload(length).await?;
                let value = String::from_utf8(value)
                    .map_err(|_| ReadError::NonAsciiPriorityUpdate)?;

                Frame::PriorityUpdate {
                    origin: PriorityUpdateOrigin::PushStream,
                    stream_or_push_id: StreamOrPushId::PushId(PushId(id)),
                    value
                }
            }
            _ => Frame::Unknown { frame_type, payload: self.read_payload(length).await? }
        })
    }

    async fn read_payload(&mut self, length: usize) -> Result<Vec<u8>, ReadError> {
        if length == 0 {
            return Ok(Vec::new());
        }

        let mut payload = Vec::new();
        payload.resize(length, 0);

        self.read_exact(&mut payload[0..length]).await?;

        Ok(payload)
    }

    async fn read_qpack_integer(&mut self, first_byte: u8, n: u32) -> Result<usize, IoError> {
        let mut i = first_byte as usize;
        if i < (2_usize.pow(n) - 1) {
            return Ok(i);
        }

        let mut m = 0;
        loop {
            let octet = self.read_u8().await?;
            i += ((octet & 0x7F) as usize) * 2_usize.pow(m);
            m += 7;

            if octet & 0x80 != 0x80 {
                return Ok(i);
            }
        }
    }

    async fn read_variable_integer(&mut self) -> Result<usize, ReadError> {
        let first_byte = self.read_u8().await?;
        let prefix = first_byte >> 6;
        let length = (1 << prefix) - 1;
        let mut value = (first_byte & 0b0011_1111) as usize;

        for _  in 0..length {
            let byte = self.read_u8().await?;
            value = (value << 8) + byte as usize;
        }

        Ok(value)
    }
}

/// How many bytes will a encoded variable-length integer take.
pub fn variable_integer_encoded_length(value: usize) -> u8 {
    match value {
        0..=0x3F => 1,
        0x40..=0x3FFF => 2,
        0x4000..=0x3FFFFFFF => 4,
        0x40000000..=0x3FFFFFFFFFFFFFFF => 8,
        0x4000000000000000..=usize::MAX => 0,
        // `usize` does not have a maximum according to the `match`ing
        // rules, but it should be covered by the previous arm.
        _ => {
            #[cfg(debug_assertions)]
            unreachable!();

            #[cfg(not(debug_assertions))]
            0
        }
    }
}

#[async_trait]
pub(super) trait WriteExtensions: AsyncWriteExt {
    async fn write_frame(&mut self, frame: Frame) -> Result<(), IoError>;
    async fn write_stream_header(&mut self, stream_type: UnidirectionalStreamType) -> Result<(), IoError>;
    async fn write_variable_integer(&mut self, value: usize) -> Result<(), IoError>;
}

#[async_trait]
impl<T> WriteExtensions for T
        where T: AsyncWriteExt + Unpin + Send {
    async fn write_frame(&mut self, frame: Frame) -> Result<(), IoError> {
        println!("[HTTP/3] Sending frame: {frame:#?}");
        self.write_variable_integer(frame.frame_type()).await?;
        self.write_variable_integer(frame.len()).await?;
        match frame {
            Frame::Data { payload } => self.write_all(&payload).await?,
            Frame::Headers { payload } => self.write_all(&payload).await?,
            Frame::CancelPush { push_id } => self.write_variable_integer(push_id.0).await?,
            Frame::Settings { settings } => {
                for (key, value) in settings.iter() {
                    self.write_variable_integer(*key as usize).await?;
                    self.write_variable_integer(*value).await?;
                }
            }
            Frame::PushPromise { push_id, payload } => {
                self.write_variable_integer(push_id.0).await?;
                self.write_all(&payload).await?;
            }
            Frame::GoAway { stream_or_push_id } => {
                debug_assert!(matches!(stream_or_push_id, StreamOrPushId::StreamId(..)));
                self.write_variable_integer(stream_or_push_id.into()).await?;
            }
            Frame::Origin { .. } => todo!(),
            Frame::MaxPushId { push_id } => self.write_variable_integer(push_id.0).await?,
            Frame::Metadata { .. } => todo!(),
            Frame::PriorityUpdate { .. } => todo!(),
            Frame::Unknown { payload, .. } => self.write_all(&payload).await?,
        }

        Ok(())
    }

    async fn write_stream_header(&mut self, stream_type: UnidirectionalStreamType) -> Result<(), IoError> {
        self.write_variable_integer(stream_type.clone().into()).await?;

        if let UnidirectionalStreamType::Push(push_id) = stream_type {
            self.write_variable_integer(push_id.0).await?;
        }

        Ok(())
    }

    async fn write_variable_integer(&mut self, value: usize) -> Result<(), IoError> {
        match value {
            0..=0x3F => self.write_u8(value as _).await?,
            0x40..=0x3FFF => self.write_u16(0b01 << 14 | value as u16).await?,
            0x4000..=0x3FFFFFFF => self.write_u32(0b10 << 30 | value as u32).await?,
            0x40000000..=0x3FFFFFFFFFFFFFFF => self.write_u64(0b11 << 62 | value as u64).await?,
            0x4000000000000000..=usize::MAX => return Err(IoError::InvalidVariableLengthInteger),
            // `usize` does not have a maximum according to the `match`ing
            // rules, but it should be covered by the previous arm.
            _ => {
                #[cfg(debug_assertions)]
                unreachable!();

                #[cfg(not(debug_assertions))]
                Err(IoError::InvalidVariableLengthInteger)
            }
        }

        Ok(())
    }
}

//
// Testing
//

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(&[0x00, 0x00], Frame::Data{ payload: Vec::new() })]
    #[case(&[0x00, 0x01, 0x69], Frame::Data{ payload: vec![0x69] })]
    #[case(&[0x01, 0x00], Frame::Headers{ payload: Vec::new() })]
    #[tokio::test]
    async fn test_read_frame(#[case] input: &[u8], #[case] expected: Frame) {
        let mut cursor = io::Cursor::new(input);
        let decoded = cursor.read_frame().await.unwrap();
        assert_eq!(decoded, expected);
    }

    #[rstest]
    #[case(0, &[0x00])]
    #[case(1, &[0x01])]
    #[case(63, &[0x3F])]
    // QUIC A.1 examples
    #[case(37, &[0x25])]
    #[case(37, &[0x40, 0x25])]
    #[case(15_293, &[0x7b, 0xbd])]
    #[case(494_878_333, &[0x9d, 0x7f, 0x3e, 0x7d])]
    #[case(151_288_809_941_952_652, &[0xc2, 0x19, 0x7c, 0x5e, 0xff, 0x14, 0xe8, 0x8c])]
    #[tokio::test]
    async fn test_read_variable_integer(#[case] expected: usize, #[case] input: &[u8]) {
        let mut cursor = io::Cursor::new(input);
        let decoded = cursor.read_variable_integer().await.unwrap();
        assert_eq!(decoded, expected);
    }

    #[rstest]
    #[case(0, &[0x00])]
    #[case(1, &[0x01])]
    #[case(63, &[0x3F])]
    // QUIC A.1 examples
    #[case(37, &[0x25])]
    #[case(15_293, &[0x7b, 0xbd])]
    #[case(494_878_333, &[0x9d, 0x7f, 0x3e, 0x7d])]
    #[case(151_288_809_941_952_652, &[0xc2, 0x19, 0x7c, 0x5e, 0xff, 0x14, 0xe8, 0x8c])]
    #[tokio::test]
    async fn test_write_variable_integer(#[case] input: usize, #[case] expected: &[u8]) {
        let mut data = Vec::with_capacity(expected.len());
        data.write_variable_integer(input).await.unwrap();
        assert_eq!(&data, expected);
    }
}
