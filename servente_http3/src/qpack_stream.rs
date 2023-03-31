// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{sync::{Arc, Weak}, collections::VecDeque};

use async_trait::async_trait;
use quinn::{
    RecvStream,
    SendStream, Connection,
};
use tokio::{sync::RwLock, io::AsyncReadExt};

use servente_http2::hpack::{
    self,
    DynamicTableEntry
};

use crate::{
    ErrorCode,
    io_extensions::ReadExtensions,
};

use super::{io_extensions::{WriteExtensions, IoError}, UnidirectionalStreamType, ConnectionNotification, ConnectionHandle};

pub struct DynamicTable {
    entries: VecDeque<(DynamicTableEntry, usize)>,
    total_size: usize,
    maximum_size: Option<usize>,
    known_received_count: usize,
}

impl DynamicTable {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            total_size: 0,
            maximum_size: None,
            known_received_count: 0,
        }
    }

    pub fn duplicate(&mut self, index: usize) -> Result<(), super::ErrorCode> {
        // If the decoder encounters a reference in an encoder instruction to a
        // dynamic table entry that has already been evicted, it MUST treat this
        // as a connection error of type QPACK_ENCODER_STREAM_ERROR.
        if self.entries.len() <= index {
            return Err(super::ErrorCode::QpackEncoderStreamError);
        }

        let entry = self.entries[index].clone();
        self.insert_with_known_size(entry.0, entry.1);

        Ok(())
    }

    /// The badge we can verify the known received count is incremented
    /// where needed, and not where it isn't correct.
    #[inline(always)]
    pub(self) fn increment_known_received_count(&mut self, _badge: KnownReceivedCountIncrementOrigin) {
        self.known_received_count += 1;
    }

    pub fn insert(&mut self, entry: DynamicTableEntry) {
        let entry_size = entry.calculate_size();
        self.insert_with_known_size(entry, entry_size);
    }

    fn insert_with_known_size(&mut self, entry: DynamicTableEntry, entry_size: usize) {
        if let Some(maximum_size) = self.maximum_size {
            // The entry will never fit in the table, so all the other entries
            // must be removed since they would be evicted anyway.
            if entry_size > maximum_size {
                self.entries.clear();
                self.total_size = 0;
                return;
            }

            // Evict other entries if the entry won't fit otherwise.
            while self.total_size + entry_size > maximum_size {
                match self.entries.pop_back() {
                    Some(entry) => {
                        self.total_size -= entry.1;
                    }
                    None => {
                        #[cfg(debug_assertions)]
                        {
                            println!("[QPACK] DynamicTable: pop_back failed total_size={} maximum_size={} entry_size={}",
                                self.total_size, maximum_size, entry_size);
                            unreachable!("See previous line, this shouldn't happen and means the data is corrupted");
                        }
                        #[cfg(not(debug_assertions))]
                        {
                            self.total_size = 0;
                            self.entries.clear();
                            return;
                        }
                    }
                }
            }

            self.entries.push_front((entry, entry_size));
        }
    }

    pub(self) fn insert_with_name_reference(&mut self, name_reference: TableIndexReference, value: String) -> Result<(), StreamError> {
        match name_reference {
            TableIndexReference::Dynamic { index } => {
                // If the decoder encounters a reference in an encoder instruction to a
                // dynamic table entry that has already been evicted, it MUST treat this
                // as a connection error of type QPACK_ENCODER_STREAM_ERROR.
                if self.entries.len() <= index {
                    return Err(StreamError::ConnectionError(super::ErrorCode::QpackEncoderStreamError));
                }

                self.insert(self.entries[index].0.with_value(value));
            }
            TableIndexReference::Static { index } => match super::static_table::TABLE.get(index) {
                None => return Err(StreamError::ConnectionError(super::ErrorCode::QpackEncoderStreamError)),
                Some(entry) => {
                    if let Some(entry) = entry.into_dynamic_table_entry_with_value_from_client(value) {
                        self.insert(entry);
                    } else {
                        // A non-client entry was sent (i.e. :path or :status)
                        return Err(StreamError::ConnectionError(super::ErrorCode::QpackEncoderStreamError));
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EncoderInstruction {
    SetDynamicTableCapacity {
        capacity: usize,
    },
    InsertWithNameReference {
        reference: TableIndexReference,
        value: String,
    },
    InsertWithLiteralName {
        name: String,
        value: String
    },
    Duplicate {
        dynamic_table_index: usize,
    }
}

/// This acts as a badge for incrementing the received count, and doesn't do
/// anything functionally.
enum KnownReceivedCountIncrementOrigin {
    DuplicateInstruction,
    InsertLiteralInstruction,
    InsertReferenceInstruction,
}

pub(crate) struct QpackDecoderSendStream {
    connection_handle: ConnectionHandle,
    stream: SendStream,
}

impl QpackDecoderSendStream {
    pub(crate) async fn create(mut stream: SendStream, connection_handle: ConnectionHandle) -> Result<Self, IoError> {
        _ = stream.write_variable_integer(UnidirectionalStreamType::QpackDecoderStream.into()).await?;
        Ok(Self {
            connection_handle,
            stream,
        })
    }
}

pub(super) struct QpackEncoderReceiveStream {
    stream: RecvStream,
    decoder_send_stream: QpackDecoderSendStream,
    connection_handle: ConnectionHandle,
}

impl QpackEncoderReceiveStream {
    pub async fn spawn(stream: RecvStream, connection_handle: ConnectionHandle) {
        let decoder_send_stream = match connection_handle.connection_info.upgrade() {
            Some(connection_info) => match connection_info.write().await.decoder_send_stream_to_take.take() {
                Some(stream) => stream,
                None => {
                    println!("[QPACK] Encoder receive stream incoming, but decoder already taken");
                    return;
                }
            }
            None => return
        };

        let decoder_send_stream = match QpackDecoderSendStream::create(decoder_send_stream, connection_handle.clone()).await {
            Ok(stream) => stream,
            Err(_) => return,
        };

        tokio::task::spawn(async move {
            let mut stream = QpackEncoderReceiveStream { stream, decoder_send_stream, connection_handle };

            match stream.start_loop().await {
                Ok(()) => (),
                Err(e) => {
                    println!("[QPACK] [EncoderRecvStream] Error: {e:?}");
                    let error_code = match e {
                        StreamError::ConnectionClosed => ErrorCode::H3ClosedStreamCritical,
                        StreamError::ConnectionError(error_code) => error_code,
                        StreamError::IoError(_) => ErrorCode::H3ClosedStreamCritical,
                    };

                    // The stream has been closed or an unrecoverable error has
                    // been encountered, so we notify the connection manager of
                    // this to act upon it, i.e. close the connection.
                    _ = stream.connection_handle.notifier.send(ConnectionNotification::ConnectionError(error_code)).await;
                }
            }
        });
    }

    async fn start_loop(&mut self) -> Result<(), StreamError> {
        loop {
            let instruction = self.stream.read_encoder_instruction().await?;
            let locked_dynamic_table = self.connection_handle.dynamic_table
                        .upgrade()
                        .ok_or(StreamError::ConnectionClosed)?;
            let mut dynamic_table = locked_dynamic_table.write().await;

            println!("[QPACK] [EncoderRecvStream] Instructed with {instruction:#?}");

            match instruction {
                EncoderInstruction::Duplicate { dynamic_table_index } => {
                    dynamic_table.increment_known_received_count(KnownReceivedCountIncrementOrigin::DuplicateInstruction);
                    dynamic_table.duplicate(dynamic_table_index)
                        .map_err(|e| StreamError::ConnectionError(e))?;
                }
                EncoderInstruction::InsertWithLiteralName { name, value } => {
                    dynamic_table.increment_known_received_count(KnownReceivedCountIncrementOrigin::InsertLiteralInstruction);
                    dynamic_table.insert(DynamicTableEntry::Header { name: name.into(), value: value.into() });
                }
                EncoderInstruction::InsertWithNameReference { reference, value } => {
                    dynamic_table.increment_known_received_count(KnownReceivedCountIncrementOrigin::InsertReferenceInstruction);
                    dynamic_table.insert_with_name_reference(reference, value)?;
                }
                _ => ()
            }
        }
    }
}

#[async_trait]
trait QpackIoExtensions {
    async fn read_encoder_instruction(&mut self) -> Result<EncoderInstruction, IoError>;

    async fn read_qpack_string(&mut self) -> Result<String, IoError>;
}

#[async_trait]
impl<T> QpackIoExtensions for T
        where T: ReadExtensions + Send + Unpin {
    async fn read_encoder_instruction(&mut self) -> Result<EncoderInstruction, IoError> {
        let first_byte = self.read_u8().await?;

        // Insert with Name Reference
        if first_byte & 0b1000_0000 == 0b1000_0000 {
            let index = self.read_qpack_integer(first_byte, 6).await?;
            let reference = if first_byte >> 6 & 1 == 1 {
                TableIndexReference::Static { index }
            } else {
                TableIndexReference::Dynamic { index }
            };

            return Ok(EncoderInstruction::InsertWithNameReference {
                reference,
                value: self.read_qpack_string().await?
            });
        }

        // Insert with Literal Name
        if first_byte & 0b0100_0000 == 0b0100_0000 {
            return Ok(EncoderInstruction::InsertWithLiteralName {
                name: self.read_qpack_string().await?,
                value: self.read_qpack_string().await?
            });
        }

        // Set Dynamic Table Capacity
        if first_byte & 0b0010_0000 == 0b0010_0000 {
            return Ok(EncoderInstruction::SetDynamicTableCapacity {
                capacity: self.read_qpack_integer(first_byte, 5).await?
            });
        }

        // Duplicate
        assert!(first_byte < 0b0010_0000);
        Ok(EncoderInstruction::Duplicate {
            dynamic_table_index: self.read_qpack_integer(first_byte, 5).await?
        })
    }

    async fn read_qpack_string(&mut self) -> Result<String, IoError> {
        let first_octet = self.read_u8().await?;

        let is_huffman = first_octet & 0x80 == 0x80;
        let length = self.read_qpack_integer(first_octet & 0x7F, 7).await?;

        let mut vec = Vec::new();
        for _ in 0..length {
            vec.push(self.read_u8().await?);
        }

        if !is_huffman {
            return String::from_utf8(vec)
                .map_err(|_| IoError::QpackUtf8Error);
        }

        hpack::decode_huffman(vec.as_slice())
            .ok_or(IoError::QpackHuffmanError)
    }
}

/// Internal errors for stream operations, to be propagated to the connection if
/// necessary.
#[derive(Debug)]
enum StreamError {
    /// The connection is believed to have closed, because of (partial) state
    /// destruction, for example an [`Arc`] being dropped and only leaving
    /// `Weak` references behind.
    ConnectionClosed,
    ConnectionError(super::ErrorCode),
    IoError(IoError),
}

impl From<IoError> for StreamError {
    fn from(value: IoError) -> Self {
        Self::IoError(value)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum TableIndexReference {
    /// Referencing an index in the static table
    Static {
        index: usize
    },
    /// Referencing an index in the dynamic table
    Dynamic {
        index: usize
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::io::Cursor;

    #[rstest]
    #[case(&[0x00], EncoderInstruction::Duplicate { dynamic_table_index: 0 })]
    #[case(&[0x20], EncoderInstruction::SetDynamicTableCapacity { capacity: 0 })]
    #[tokio::test]
    async fn test_read_encoder_instruction(#[case] input: &[u8], #[case] expected: EncoderInstruction) {
        let mut stream = Cursor::new(input);
        let instruction = stream.read_encoder_instruction().await.unwrap();
        assert_eq!(instruction, expected);
    }
}
