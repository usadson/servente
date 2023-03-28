// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! HPACK: Header Compression for HTTP/2
//!
//! HPACK is the compression format for efficiently representing HTTP headers
//! in HTTP/2.
//!
//! # References
//! * [RFC 7541](https://httpwg.org/specs/rfc7541.html)

use std::{
    collections::VecDeque,
    io::Write,
    sync::Arc
};

use tokio::sync::Mutex;

use servente_http::{
    HeaderName,
    HeaderMap,
    HeaderValue,
    Method,
    Request,
    Response,
    StatusCode, HttpVersion, RequestTarget,
};

/// HPACK write extensions for [`Write`] objects
trait WriteExtensions: Write {
    /// Write a number in the HPACK format.
    fn write_hpack_number(&mut self, value: usize, n: u8, prefix: u8) -> Result<(), std::io::Error>;

    /// Write a string in Huffman representation.
    fn write_hpack_string_huffman(&mut self, value: &str) -> Result<(), std::io::Error>;
}

fn calculate_string_length_in_huffman_bytes(value: &str) -> usize {
    // Calculate the length in huffman bits
    let len: usize = value.as_bytes().iter().map(|b| HUFFMAN_CODE[*b as usize].length_in_bits as usize).sum();
    // Convert from bits into bytes, taking padding into account.
    if len % 8 == 0 {
        len / 8
    } else {
        (len + (8 - (len % 8))) / 8
    }
}

impl<T> WriteExtensions for T where T: Write {
    fn write_hpack_number(&mut self, value: usize, n: u8, prefix: u8) -> Result<(), std::io::Error> {
        let first_octet_max = 2_usize.pow(n as _) - 1;

        if value < first_octet_max {
            self.write_all(&[prefix | value as u8])?;
            return Ok(());
        }

        self.write_all(&[prefix | first_octet_max as u8])?;
        let mut value = value - first_octet_max;
        while value >= 128 {
            self.write_all(&[value as u8 % 128 + 128])?;
            value /= 128;
        }
        self.write_all(&[value as _])?;

        Ok(())
    }

    fn write_hpack_string_huffman(&mut self, value: &str) -> Result<(), std::io::Error> {
        let len = calculate_string_length_in_huffman_bytes(value);

        // Write with the 'H' flag set.
        self.write_hpack_number(len, 7, 0x80)?;
        let mut writer = BitWriter::new(self);

        for byte in value.as_bytes() {
            let entry = HUFFMAN_CODE[*byte as usize];
            let mut cycles = 0;
            for bit in BitReader3::new(entry.code, entry.length_in_bits) {
                writer.push(bit)?;
                cycles += 1;
            }

            debug_assert_eq!(cycles, entry.length_in_bits);
        }

        Ok(())
    }
}

fn compress_status_code(data: &mut Vec<u8>, status_code: StatusCode) {
    for (idx, entry) in STATIC_TABLE.iter().enumerate() {
        if let StaticTableEntry::Status(static_status) = entry {
            if *static_status == status_code {
                data.push(0x80 | idx as u8);
                return;
            }
        }
    }

    // Literal Header Field without Indexing — Indexed Name
    data.push(8);
    _ = data.write_hpack_number(3, 7, 0);
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CompressIndexCandidate {
    None,
    FullyIndexed(usize),
    NameIndexed(usize),
}

#[derive(Debug)]
pub struct Compressor {

}

impl Compressor {
    pub fn new() -> Self {
        Self {

        }
    }

    fn find_header(&self, header_name: &HeaderName, header_value: &HeaderValue, header_value_as_str: &str) -> CompressIndexCandidate {
        let mut candidate = CompressIndexCandidate::None;
        for (index, entry) in STATIC_TABLE.iter().enumerate() {
            match entry {
                StaticTableEntry::Header(name) => {
                    if candidate != CompressIndexCandidate::None {
                        continue;
                    }

                    if header_name == name || header_name.to_string_h1().eq_ignore_ascii_case(name.to_string_h1()) {
                        candidate = CompressIndexCandidate::NameIndexed(index);
                    }
                },
                StaticTableEntry::HeaderWithValue { name, value } => {
                    if header_name == name || header_name.to_string_h1().eq_ignore_ascii_case(name.to_string_h1()) {
                        if value == header_value || value.to_string().eq_ignore_ascii_case(header_value_as_str) {
                            return CompressIndexCandidate::FullyIndexed(index);
                        }
                        candidate = CompressIndexCandidate::NameIndexed(index);
                    }
                }
                _ => (),
            };
        }

        candidate
    }

    pub fn compress(&mut self, response: &Response) -> Vec<u8> {
        let mut data = Vec::new();

        compress_status_code(&mut data, response.status);
        for (header_name, header_value) in response.headers.iter() {
            let header_value_as_str = header_value.to_string();
            match self.find_header(header_name, header_value, &header_value_as_str) {
                CompressIndexCandidate::None => {
                    // Literal Header Field without Indexing — New Name
                    data.push(0x00);
                    _ = data.write_hpack_string_huffman(&header_name.to_string_lowercase());
                    _ = data.write_hpack_string_huffman(&header_value_as_str);
                }
                CompressIndexCandidate::NameIndexed(index) => {
                    // Literal Header Field without Indexing — Indexed Name
                    _ = data.write_hpack_number(index, 4, 0);
                    _ = data.write_hpack_string_huffman(&header_value_as_str);
                }
                CompressIndexCandidate::FullyIndexed(index) => {
                    _ = data.write_hpack_number(index, 7, 0x80);
                }
            }
        }

        data
    }
}

// TODO: some `error`s MUST be conveyed by a `COMPRESSION_ERROR`, and some MUST
//       be with a `400 Bad Request`. `COMPRESSION_ERROR` is now used as the
//       only way of feedback.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DecompressionError {
    LookupError(DynamicTableLookupError),
    NoPath,
    NoMethod,
    NoScheme,
    UnexpectedEndOfFile,
    DynamicTableUpdateTooLarge,
    DynamicTableUpdateNotFirst,

    DuplicateAuthority,
    DuplicateMethod,
    DuplicatePath,
    DuplicateScheme,

    PseudoAfterRegularFields,

    EmptyPath,

    FieldNameEmpty,
    FieldNameInvalidNonVisibleAscii,
    FieldNameInvalidAsciiSpace,
    FieldNameInvalidUppercase,
    FieldNameStartWithColonNonPseudoField,
    FieldNameExtendedAsciiUnicode,

    FieldValueContainsNul,
    FieldValueContainsCarriageReturn,
    FieldValueContainsLineFeed,
    FieldValueStartsWithWhitespace,
    FieldValueEndsWithWhitespace,

    /// HTTP/2 does not use the headers conveying connection-specific semantics
    /// of text-based HTTP versions (e.g. HTTP/1.1), since they are represented
    /// as core parts of the protocol.
    ///
    /// # Examples
    /// In **HTTP/1.1**, the following might be transmitted:
    /// ```text
    /// Connection: keep-alive
    /// ```
    /// since connections might or might not be kept alive after a response has
    /// been sent. **HTTP/2** does not have this limitation, since connections
    /// must be expliticly terminated with a `GOAWAY` frame.
    ///
    /// # References
    /// * [RFC 9113 - Section 8.2.2](https://httpwg.org/specs/rfc9113.html#ConnectionSpecific)
    ConnectionSpecificHeaderField,

    /// An exception to the [ConnectionSpecificHeaderField] is the `TE` header,
    /// and it may only contain `trailers`.
    ///
    /// # References
    /// * [RFC 9113 - Section 8.2.2](https://httpwg.org/specs/rfc9113.html#ConnectionSpecific)
    TeHeaderNotTrailers,
}

impl From<DynamicTableLookupError> for DecompressionError {
    fn from(value: DynamicTableLookupError) -> Self {
        Self::LookupError(value)
    }
}

#[derive(Debug)]
pub struct DynamicTable {
    table: VecDeque<(DynamicTableEntry, usize)>,
    current_size: usize,
    max_size: usize,
}

impl DynamicTable {
    pub fn new(max_size: usize) -> Self {
        Self {
            table: VecDeque::new(),
            current_size: 0,
            max_size,
        }
    }

    /// The static table and the dynamic table are combined into a single index address space.
    ///
    /// # References
    /// * [RFC 7541 - Section 2.3.3](https://httpwg.org/specs/rfc7541.html#index.address.space)
    pub fn get(&self, index: usize, supplied_value: Option<String>) -> Result<DynamicTableEntry, DynamicTableLookupError> {
        if index < STATIC_TABLE.len() {
            return match STATIC_TABLE[index].clone() {
                StaticTableEntry::Illegal => Err(DynamicTableLookupError::InvalidIndex),
                StaticTableEntry::Authority => {
                    if let Some(value) = supplied_value {
                        Ok(DynamicTableEntry::Authority(value.into()))
                    } else {
                        Err(DynamicTableLookupError::PseudoHeaderWithoutValue)
                    }
                }
                StaticTableEntry::Method(method) => {
                    if let Some(value) = supplied_value {
                        Ok(DynamicTableEntry::Method(Method::from(value)))
                    } else {
                        Ok(DynamicTableEntry::Method(method))
                    }
                }
                StaticTableEntry::Path(path) => {
                    if let Some(value) = supplied_value {
                        Ok(DynamicTableEntry::Path(value.into()))
                    } else {
                        Ok(DynamicTableEntry::Path(path.into()))
                    }
                }
                StaticTableEntry::Scheme(scheme) => {
                    if let Some(value) = supplied_value {
                        Ok(DynamicTableEntry::Scheme(value.into()))
                    } else {
                        Ok(DynamicTableEntry::Scheme(scheme.into()))
                    }
                }
                StaticTableEntry::Status(_) => Err(DynamicTableLookupError::PseudoHeaderStatus),
                StaticTableEntry::Header(name) => {
                    if let Some(value) = supplied_value {
                        Ok(DynamicTableEntry::Header { name, value: value.into() })
                    } else {
                        Ok(DynamicTableEntry::Header { name, value: "".into() })
                    }
                }
                StaticTableEntry::HeaderWithValue { name, value } => {
                    if let Some(value) = supplied_value {
                        return Ok(DynamicTableEntry::Header { name, value: value.into() });
                    } else {
                        return Ok(DynamicTableEntry::Header { name, value });
                    }
                }
            };
        }

        match self.table.get(index - STATIC_TABLE.len()) {
            Some((entry, _)) => match supplied_value {
                Some(value) => Ok(entry.with_value(value)),
                None => Ok(entry.clone()),
            }
            None => Err(DynamicTableLookupError::OutOfBounds),
        }
    }

    pub fn insert(&mut self, entry: DynamicTableEntry) {
        let entry_size = entry.calculate_size();
        if self.max_size < entry_size {
            self.table.clear();
            return;
        }

        while self.current_size + entry_size > self.max_size {
            let Some((_, last_entry_size)) = self.table.pop_back() else {
                // Table is empty, so the entry will never fit in the table.
                return;
            };

            self.current_size -= last_entry_size;
        }

        self.table.push_front((entry, entry_size));
    }

    pub fn size_update(&mut self, size: usize) {
        while self.current_size > size {
            let Some((_, last_entry_size)) = self.table.pop_back() else {
                // Table is empty, so it will never fit in the table.
                return;
            };

            self.current_size -= last_entry_size;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DynamicTableEntry {
    Authority(StaticOrSharedString),
    Header {
        name: HeaderName,
        value: HeaderValue
    },
    Method(Method),
    Path(StaticOrSharedString),
    Scheme(StaticOrSharedString),
}

impl DynamicTableEntry {
    /// Calculates the size of this entry, as defined by
    /// [`HPACK`](https://httpwg.org/specs/rfc7541.html).
    ///
    /// It is equal to the sum of: length of the name, the length of the value,
    /// and 32.
    ///
    /// # Examples
    /// Given the following example from `RFC 7541`:
    /// ```text
    /// [  1] (s =  57) :authority: www.example.com
    ///       Table size:  57
    /// ```
    ///
    /// As represented by the following Servente code:
    /// ```ignore
    /// use servente_http2::hpack::DynamicTableEntry;
    /// let entry = DynamicTableEntry::Authority("www.example.com".into());
    /// let size = entry.calculate_size();
    /// assert_eq!(size, 57);
    /// ```
    pub fn calculate_size(&self) -> usize {
        let (name_len, value_len) = match self {
            Self::Authority(str) => (":authority".len(), str.as_ref().len()),
            Self::Header { name, value } => (name.to_string_h1().len(), value.string_length()),
            Self::Method(method) => (":method".len(), method.as_string().len()), // TODO!
            Self::Path(str) => (":path".len(), str.as_ref().len()),
            Self::Scheme(str) => (":scheme".len(), str.as_ref().len()),
        };

        name_len + value_len + 32
    }

    pub fn with_value(&self, value: String) -> Self {
        match self {
            Self::Authority(_) => Self::Authority(value.into()),
            Self::Header { name, .. } => Self::Header { name: name.clone(), value: HeaderValue::from(value) },
            Self::Method(_) => Self::Method(value.into()),
            Self::Path(_) => Self::Path(value.into()),
            Self::Scheme(_) => Self::Scheme(value.into()),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DynamicTableLookupError {
    /// Index 0 is illegal.
    InvalidIndex,

    /// The field doesn't exist.
    OutOfBounds,

    /// Pseudo-header should've been supplied with a value.
    PseudoHeaderWithoutValue,

    /// Pseudo-header was ':status', the response pseudo-header.
    PseudoHeaderStatus,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct HuffmanEntry {
    code: u32,
    length_in_bits: u8,
}

impl HuffmanEntry {
    const fn new(code: u32, length_in_bits: u8) -> Self {
        Self { code, length_in_bits }
    }
}

/// The Huffman code, as defined by HPACK.
const HUFFMAN_CODE: &[HuffmanEntry] = &[
    HuffmanEntry::new(0x1ff8, 13),
    HuffmanEntry::new(0x7fffd8, 23),
    HuffmanEntry::new(0xfffffe2, 28),
    HuffmanEntry::new(0xfffffe3, 28),
    HuffmanEntry::new(0xfffffe4, 28),
    HuffmanEntry::new(0xfffffe5, 28),
    HuffmanEntry::new(0xfffffe6, 28),
    HuffmanEntry::new(0xfffffe7, 28),
    HuffmanEntry::new(0xfffffe8, 28),
    HuffmanEntry::new(0xffffea, 24),
    HuffmanEntry::new(0x3ffffffc, 30),
    HuffmanEntry::new(0xfffffe9, 28),
    HuffmanEntry::new(0xfffffea, 28),
    HuffmanEntry::new(0x3ffffffd, 30),
    HuffmanEntry::new(0xfffffeb, 28),
    HuffmanEntry::new(0xfffffec, 28),
    HuffmanEntry::new(0xfffffed, 28),
    HuffmanEntry::new(0xfffffee, 28),
    HuffmanEntry::new(0xfffffef, 28),
    HuffmanEntry::new(0xffffff0, 28),
    HuffmanEntry::new(0xffffff1, 28),
    HuffmanEntry::new(0xffffff2, 28),
    HuffmanEntry::new(0x3ffffffe, 30),
    HuffmanEntry::new(0xffffff3, 28),
    HuffmanEntry::new(0xffffff4, 28),

    HuffmanEntry::new(0xffffff5, 28),
    HuffmanEntry::new(0xffffff6, 28),
    HuffmanEntry::new(0xffffff7, 28),
    HuffmanEntry::new(0xffffff8, 28),
    HuffmanEntry::new(0xffffff9, 28),
    HuffmanEntry::new(0xffffffa, 28),
    HuffmanEntry::new(0xffffffb, 28),
    HuffmanEntry::new(0x14, 6),
    HuffmanEntry::new(0x3f8, 10),
    HuffmanEntry::new(0x3f9, 10),
    HuffmanEntry::new(0xffa, 12),
    HuffmanEntry::new(0x1ff9, 13),
    HuffmanEntry::new(0x15, 6),
    HuffmanEntry::new(0xf8, 8),
    HuffmanEntry::new(0x7fa, 11),
    HuffmanEntry::new(0x3fa, 10),
    HuffmanEntry::new(0x3fb, 10),
    HuffmanEntry::new(0xf9, 8),
    HuffmanEntry::new(0x7fb, 11),
    HuffmanEntry::new(0xfa, 8),
    HuffmanEntry::new(0x16, 6),
    HuffmanEntry::new(0x17, 6),
    HuffmanEntry::new(0x18, 6),
    HuffmanEntry::new(0x0, 5),
    HuffmanEntry::new(0x1, 5),
    HuffmanEntry::new(0x2, 5),
    HuffmanEntry::new(0x19, 6),
    HuffmanEntry::new(0x1a, 6),
    HuffmanEntry::new(0x1b, 6),
    HuffmanEntry::new(0x1c, 6),
    HuffmanEntry::new(0x1d, 6),
    HuffmanEntry::new(0x1e, 6),
    HuffmanEntry::new(0x1f, 6),
    HuffmanEntry::new(0x5c, 7),
    HuffmanEntry::new(0xfb, 8),
    HuffmanEntry::new(0x7ffc, 15),
    HuffmanEntry::new(0x20, 6),
    HuffmanEntry::new(0xffb, 12),
    HuffmanEntry::new(0x3fc, 10),
    HuffmanEntry::new(0x1ffa, 13),
    HuffmanEntry::new(0x21, 6),
    HuffmanEntry::new(0x5d, 7),
    HuffmanEntry::new(0x5e, 7),
    HuffmanEntry::new(0x5f, 7),
    HuffmanEntry::new(0x60, 7),
    HuffmanEntry::new(0x61, 7),
    HuffmanEntry::new(0x62, 7),
    HuffmanEntry::new(0x63, 7),
    HuffmanEntry::new(0x64, 7),
    HuffmanEntry::new(0x65, 7),
    HuffmanEntry::new(0x66, 7),
    HuffmanEntry::new(0x67, 7),
    HuffmanEntry::new(0x68, 7),
    HuffmanEntry::new(0x69, 7),
    HuffmanEntry::new(0x6a, 7),
    HuffmanEntry::new(0x6b, 7),
    HuffmanEntry::new(0x6c, 7),
    HuffmanEntry::new(0x6d, 7),
    HuffmanEntry::new(0x6e, 7),
    HuffmanEntry::new(0x6f, 7),
    HuffmanEntry::new(0x70, 7),
    HuffmanEntry::new(0x71, 7),
    HuffmanEntry::new(0x72, 7),
    HuffmanEntry::new(0xfc, 8),
    HuffmanEntry::new(0x73, 7),
    HuffmanEntry::new(0xfd, 8),
    HuffmanEntry::new(0x1ffb, 13),
    HuffmanEntry::new(0x7fff0, 19),
    HuffmanEntry::new(0x1ffc, 13),
    HuffmanEntry::new(0x3ffc, 14),
    HuffmanEntry::new(0x22, 6),
    HuffmanEntry::new(0x7ffd, 15),
    HuffmanEntry::new(0x3, 5),
    HuffmanEntry::new(0x23, 6),
    HuffmanEntry::new(0x4, 5),
    HuffmanEntry::new(0x24, 6),
    HuffmanEntry::new(0x5, 5),
    HuffmanEntry::new(0x25, 6),
    HuffmanEntry::new(0x26, 6),
    HuffmanEntry::new(0x27, 6),
    HuffmanEntry::new(0x6, 5),
    HuffmanEntry::new(0x74, 7),
    HuffmanEntry::new(0x75, 7),
    HuffmanEntry::new(0x28, 6),
    HuffmanEntry::new(0x29, 6),
    HuffmanEntry::new(0x2a, 6),
    HuffmanEntry::new(0x7, 5),
    HuffmanEntry::new(0x2b, 6),
    HuffmanEntry::new(0x76, 7),
    HuffmanEntry::new(0x2c, 6),
    HuffmanEntry::new(0x8, 5),
    HuffmanEntry::new(0x9, 5),
    HuffmanEntry::new(0x2d, 6),
    HuffmanEntry::new(0x77, 7),
    HuffmanEntry::new(0x78, 7),
    HuffmanEntry::new(0x79, 7),
    HuffmanEntry::new(0x7a, 7),
    HuffmanEntry::new(0x7b, 7),
    HuffmanEntry::new(0x7ffe, 15),
    HuffmanEntry::new(0x7fc, 11),
    HuffmanEntry::new(0x3ffd, 14),
    HuffmanEntry::new(0x1ffd, 13),
    HuffmanEntry::new(0xffffffc, 28),
    HuffmanEntry::new(0xfffe6, 20),
    HuffmanEntry::new(0x3fffd2, 22),
    HuffmanEntry::new(0xfffe7, 20),
    HuffmanEntry::new(0xfffe8, 20),
    HuffmanEntry::new(0x3fffd3, 22),
    HuffmanEntry::new(0x3fffd4, 22),
    HuffmanEntry::new(0x3fffd5, 22),
    HuffmanEntry::new(0x7fffd9, 23),
    HuffmanEntry::new(0x3fffd6, 22),
    HuffmanEntry::new(0x7fffda, 23),
    HuffmanEntry::new(0x7fffdb, 23),
    HuffmanEntry::new(0x7fffdc, 23),
    HuffmanEntry::new(0x7fffdd, 23),
    HuffmanEntry::new(0x7fffde, 23),
    HuffmanEntry::new(0xffffeb, 24),
    HuffmanEntry::new(0x7fffdf, 23),
    HuffmanEntry::new(0xffffec, 24),
    HuffmanEntry::new(0xffffed, 24),
    HuffmanEntry::new(0x3fffd7, 22),
    HuffmanEntry::new(0x7fffe0, 23),
    HuffmanEntry::new(0xffffee, 24),
    HuffmanEntry::new(0x7fffe1, 23),
    HuffmanEntry::new(0x7fffe2, 23),
    HuffmanEntry::new(0x7fffe3, 23),
    HuffmanEntry::new(0x7fffe4, 23),
    HuffmanEntry::new(0x1fffdc, 21),
    HuffmanEntry::new(0x3fffd8, 22),
    HuffmanEntry::new(0x7fffe5, 23),
    HuffmanEntry::new(0x3fffd9, 22),
    HuffmanEntry::new(0x7fffe6, 23),
    HuffmanEntry::new(0x7fffe7, 23),
    HuffmanEntry::new(0xffffef, 24),
    HuffmanEntry::new(0x3fffda, 22),
    HuffmanEntry::new(0x1fffdd, 21),
    HuffmanEntry::new(0xfffe9, 20),
    HuffmanEntry::new(0x3fffdb, 22),
    HuffmanEntry::new(0x3fffdc, 22),
    HuffmanEntry::new(0x7fffe8, 23),
    HuffmanEntry::new(0x7fffe9, 23),
    HuffmanEntry::new(0x1fffde, 21),
    HuffmanEntry::new(0x7fffea, 23),
    HuffmanEntry::new(0x3fffdd, 22),
    HuffmanEntry::new(0x3fffde, 22),
    HuffmanEntry::new(0xfffff0, 24),
    HuffmanEntry::new(0x1fffdf, 21),
    HuffmanEntry::new(0x3fffdf, 22),
    HuffmanEntry::new(0x7fffeb, 23),
    HuffmanEntry::new(0x7fffec, 23),
    HuffmanEntry::new(0x1fffe0, 21),
    HuffmanEntry::new(0x1fffe1, 21),
    HuffmanEntry::new(0x3fffe0, 22),
    HuffmanEntry::new(0x1fffe2, 21),
    HuffmanEntry::new(0x7fffed, 23),
    HuffmanEntry::new(0x3fffe1, 22),
    HuffmanEntry::new(0x7fffee, 23),
    HuffmanEntry::new(0x7fffef, 23),
    HuffmanEntry::new(0xfffea, 20),
    HuffmanEntry::new(0x3fffe2, 22),
    HuffmanEntry::new(0x3fffe3, 22),
    HuffmanEntry::new(0x3fffe4, 22),
    HuffmanEntry::new(0x7ffff0, 23),
    HuffmanEntry::new(0x3fffe5, 22),
    HuffmanEntry::new(0x3fffe6, 22),
    HuffmanEntry::new(0x7ffff1, 23),
    HuffmanEntry::new(0x3ffffe0, 26),
    HuffmanEntry::new(0x3ffffe1, 26),
    HuffmanEntry::new(0xfffeb, 20),
    HuffmanEntry::new(0x7fff1, 19),
    HuffmanEntry::new(0x3fffe7, 22),
    HuffmanEntry::new(0x7ffff2, 23),
    HuffmanEntry::new(0x3fffe8, 22),
    HuffmanEntry::new(0x1ffffec, 25),
    HuffmanEntry::new(0x3ffffe2, 26),
    HuffmanEntry::new(0x3ffffe3, 26),
    HuffmanEntry::new(0x3ffffe4, 26),
    HuffmanEntry::new(0x7ffffde, 27),
    HuffmanEntry::new(0x7ffffdf, 27),
    HuffmanEntry::new(0x3ffffe5, 26),
    HuffmanEntry::new(0xfffff1, 24),
    HuffmanEntry::new(0x1ffffed, 25),
    HuffmanEntry::new(0x7fff2, 19),
    HuffmanEntry::new(0x1fffe3, 21),
    HuffmanEntry::new(0x3ffffe6, 26),
    HuffmanEntry::new(0x7ffffe0, 27),
    HuffmanEntry::new(0x7ffffe1, 27),
    HuffmanEntry::new(0x3ffffe7, 26),
    HuffmanEntry::new(0x7ffffe2, 27),
    HuffmanEntry::new(0xfffff2, 24),
    HuffmanEntry::new(0x1fffe4, 21),
    HuffmanEntry::new(0x1fffe5, 21),
    HuffmanEntry::new(0x3ffffe8, 26),
    HuffmanEntry::new(0x3ffffe9, 26),
    HuffmanEntry::new(0xffffffd, 28),
    HuffmanEntry::new(0x7ffffe3, 27),
    HuffmanEntry::new(0x7ffffe4, 27),
    HuffmanEntry::new(0x7ffffe5, 27),
    HuffmanEntry::new(0xfffec, 20),
    HuffmanEntry::new(0xfffff3, 24),
    HuffmanEntry::new(0xfffed, 20),
    HuffmanEntry::new(0x1fffe6, 21),
    HuffmanEntry::new(0x3fffe9, 22),
    HuffmanEntry::new(0x1fffe7, 21),
    HuffmanEntry::new(0x1fffe8, 21),
    HuffmanEntry::new(0x7ffff3, 23),
    HuffmanEntry::new(0x3fffea, 22),
    HuffmanEntry::new(0x3fffeb, 22),
    HuffmanEntry::new(0x1ffffee, 25),
    HuffmanEntry::new(0x1ffffef, 25),
    HuffmanEntry::new(0xfffff4, 24),
    HuffmanEntry::new(0xfffff5, 24),
    HuffmanEntry::new(0x3ffffea, 26),
    HuffmanEntry::new(0x7ffff4, 23),
    HuffmanEntry::new(0x3ffffeb, 26),
    HuffmanEntry::new(0x7ffffe6, 27),
    HuffmanEntry::new(0x3ffffec, 26),
    HuffmanEntry::new(0x3ffffed, 26),
    HuffmanEntry::new(0x7ffffe7, 27),
    HuffmanEntry::new(0x7ffffe8, 27),
    HuffmanEntry::new(0x7ffffe9, 27),
    HuffmanEntry::new(0x7ffffea, 27),
    HuffmanEntry::new(0x7ffffeb, 27),
    HuffmanEntry::new(0xffffffe, 28),
    HuffmanEntry::new(0x7ffffec, 27),
    HuffmanEntry::new(0x7ffffed, 27),
    HuffmanEntry::new(0x7ffffee, 27),
    HuffmanEntry::new(0x7ffffef, 27),
    HuffmanEntry::new(0x7fffff0, 27),
    HuffmanEntry::new(0x3ffffee, 26),
];

const HUFFMAN_EOS_ENTRY: HuffmanEntry = HuffmanEntry::new(0x3fffffff, 30);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum HuffmanValue {
    EndOfStream,
    Symbol(u8),
}

struct HuffmanTree {
    table: hashbrown::HashMap<u8, hashbrown::HashMap<u32, HuffmanValue>>,
}

impl HuffmanTree {
    pub fn new() -> Self {
        let mut tree = Self {
            table: Default::default()
        };

        for (symbol, entry) in HUFFMAN_CODE.iter().enumerate() {
            let old_entry = tree.table.entry(entry.length_in_bits)
                .or_default()
                .insert(entry.code, HuffmanValue::Symbol(symbol as _));

            debug_assert!(old_entry.is_none());
            _ = old_entry;
        }

        // EOS
        let old_entry = tree.table.entry(HUFFMAN_EOS_ENTRY.length_in_bits)
            .or_default()
            .insert(HUFFMAN_EOS_ENTRY.code, HuffmanValue::EndOfStream);

        debug_assert!(old_entry.is_none());
        _ = old_entry;

        tree
    }
}

lazy_static::lazy_static! {
    //static ref HUFFMAN_TREE: Box<BinaryTreeNode> = BinaryTreeNode::construct(HUFFMAN_CODE);
    static ref HUFFMAN_TREE: HuffmanTree = HuffmanTree::new();
}

#[derive(Clone, Debug)]
pub enum StaticOrSharedString {
    Static(&'static str),
    Shared(Arc<str>),
}

impl From<&'static str> for StaticOrSharedString {
    fn from(value: &'static str) -> Self {
        Self::Static(value)
    }
}

impl From<String> for StaticOrSharedString {
    fn from(value: String) -> Self {
        Self::Shared(Arc::from(value))
    }
}

impl AsRef<str> for StaticOrSharedString {
    fn as_ref(&self) -> &str {
        match self {
            StaticOrSharedString::Static(str) => str,
            StaticOrSharedString::Shared(str) => str.as_ref(),
        }
    }
}

impl PartialEq for StaticOrSharedString {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum StaticTableEntry {
    Illegal,
    Authority,
    Method(Method),
    Path(&'static str),
    Scheme(&'static str),
    Status(StatusCode),
    Header(HeaderName),
    HeaderWithValue {
        name: HeaderName,
        value: HeaderValue,
    }
}

/// # References
/// * [RFC 7541 - Appendix A. Static Table Definition](https://httpwg.org/specs/rfc7541.html#static.table.definition)
const STATIC_TABLE: &[StaticTableEntry; 62] = &[
    StaticTableEntry::Illegal,
    StaticTableEntry::Authority,
    StaticTableEntry::Method(Method::Get),
    StaticTableEntry::Method(Method::Post),
    StaticTableEntry::Path("/"),
    StaticTableEntry::Path("/index.html"),
    StaticTableEntry::Scheme("http"),
    StaticTableEntry::Scheme("https"),
    StaticTableEntry::Status(StatusCode::Ok),
    StaticTableEntry::Status(StatusCode::NoContent),
    StaticTableEntry::Status(StatusCode::PartialContent),
    StaticTableEntry::Status(StatusCode::NotModified),
    StaticTableEntry::Status(StatusCode::BadRequest),
    StaticTableEntry::Status(StatusCode::NotFound),
    StaticTableEntry::Status(StatusCode::InternalServerError),
    StaticTableEntry::Header(HeaderName::AcceptCharset),
    StaticTableEntry::HeaderWithValue { name: HeaderName::AcceptEncoding, value: HeaderValue::StaticString("gzip, deflate") },
    StaticTableEntry::Header(HeaderName::AcceptLanguage),
    StaticTableEntry::Header(HeaderName::AcceptRanges),
    StaticTableEntry::Header(HeaderName::Accept),
    StaticTableEntry::Header(HeaderName::AccessControlAllowOrigin),
    StaticTableEntry::Header(HeaderName::Age),
    StaticTableEntry::Header(HeaderName::Allow),
    StaticTableEntry::Header(HeaderName::Authorization),
    StaticTableEntry::Header(HeaderName::CacheControl),
    StaticTableEntry::Header(HeaderName::ContentDisposition),
    StaticTableEntry::Header(HeaderName::ContentEncoding),
    StaticTableEntry::Header(HeaderName::ContentLanguage),
    StaticTableEntry::Header(HeaderName::ContentLength),
    StaticTableEntry::Header(HeaderName::ContentLocation),
    StaticTableEntry::Header(HeaderName::ContentRange),
    StaticTableEntry::Header(HeaderName::ContentType),
    StaticTableEntry::Header(HeaderName::Cookie),
    StaticTableEntry::Header(HeaderName::Date),
    StaticTableEntry::Header(HeaderName::ETag),
    StaticTableEntry::Header(HeaderName::Expect),
    StaticTableEntry::Header(HeaderName::Expires),
    StaticTableEntry::Header(HeaderName::From),
    StaticTableEntry::Header(HeaderName::Host),
    StaticTableEntry::Header(HeaderName::IfMatch),
    StaticTableEntry::Header(HeaderName::IfModifiedSince),
    StaticTableEntry::Header(HeaderName::IfNoneMatch),
    StaticTableEntry::Header(HeaderName::IfRange),
    StaticTableEntry::Header(HeaderName::IfUnmodifiedSince),
    StaticTableEntry::Header(HeaderName::LastModified),
    StaticTableEntry::Header(HeaderName::Link),
    StaticTableEntry::Header(HeaderName::Location),
    StaticTableEntry::Header(HeaderName::MaxForwards),
    StaticTableEntry::Header(HeaderName::ProxyAuthenticate),
    StaticTableEntry::Header(HeaderName::ProxyAuthorization),
    StaticTableEntry::Header(HeaderName::Range),
    StaticTableEntry::Header(HeaderName::Referer),
    StaticTableEntry::Header(HeaderName::Refresh),
    StaticTableEntry::Header(HeaderName::RetryAfter),
    StaticTableEntry::Header(HeaderName::Server),
    StaticTableEntry::Header(HeaderName::SetCookie),
    StaticTableEntry::Header(HeaderName::StrictTransportSecurity),
    StaticTableEntry::Header(HeaderName::TransferEncoding),
    StaticTableEntry::Header(HeaderName::UserAgent),
    StaticTableEntry::Header(HeaderName::Vary),
    StaticTableEntry::Header(HeaderName::Via),
    StaticTableEntry::Header(HeaderName::WwwAuthenticate),
];

pub(super) async fn decode_hpack(mut request: super::BinaryRequest, dynamic_table: Arc<Mutex<DynamicTable>>) -> Result<Request, DecompressionError> {
    let mut dynamic_table = dynamic_table.lock_owned().await;
    let mut path = None;
    let mut method = None;
    let mut scheme = None;
    let mut authority = None;
    let mut headers = Vec::new();

    let mut is_first = true;
    while let Some(first_octet) = request.read_u8() {
        let is_first = {
            let was = is_first;
            is_first = false;
            was
        };
        // 6.1. Indexed Header Field Representation
        if first_octet & 0x80 == 0x80 {
            let Some(index) = request.read_integer(first_octet & 0x7F, 7) else {
                return Err(DecompressionError::UnexpectedEndOfFile);
            };

            match dynamic_table.get(index, None)? {
                DynamicTableEntry::Authority(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if authority.is_some() {
                        return Err(DecompressionError::DuplicateAuthority);
                    }
                    authority = Some(val);
                },
                DynamicTableEntry::Header { name, value } => headers.push((name, value)),
                DynamicTableEntry::Method(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if method.is_some() {
                        return Err(DecompressionError::DuplicateMethod);
                    }
                    method = Some(val);
                }
                DynamicTableEntry::Path(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if path.is_some() {
                        return Err(DecompressionError::DuplicatePath);
                    }
                    path = Some(val.as_ref().to_string());
                }
                DynamicTableEntry::Scheme(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if scheme.is_some() {
                        return Err(DecompressionError::DuplicateScheme);
                    }
                    scheme = Some(val);
                },
            }
            continue;
        }

        // 6.2.1. Literal Header Field with Incremental Indexing
        if first_octet & 0x40 == 0x40 {
            // Literal Header Field with Incremental Indexing — New Name
            if first_octet == 0x40 {
                let name = request.read_string().ok_or(DecompressionError::UnexpectedEndOfFile)?;
                validate_header_name(&name)?;

                let value = request.read_string().ok_or(DecompressionError::UnexpectedEndOfFile)?;
                validate_header_value(&value)?;

                let header = (HeaderName::from(name), HeaderValue::from(value));
                validate_header(&header)?;
                dynamic_table.insert(DynamicTableEntry::Header { name: header.0.clone(), value: header.1.clone() });
                headers.push(header);
                continue;
            }

            // Literal Header Field with Incremental Indexing — Indexed Name
            let Some(index) = request.read_integer(first_octet & 0x3F, 6) else {
                return Err(DecompressionError::UnexpectedEndOfFile);
            };

            let value = request.read_string().ok_or(DecompressionError::UnexpectedEndOfFile)?;
            match dynamic_table.get(index, Some(value))? {
                DynamicTableEntry::Authority(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if authority.is_some() {
                        return Err(DecompressionError::DuplicateAuthority);
                    }
                    dynamic_table.insert(DynamicTableEntry::Authority(val.clone()));
                    authority = Some(val);
                },
                DynamicTableEntry::Header { name, value } => {
                    let header = (name, value);
                    dynamic_table.insert(DynamicTableEntry::Header { name: header.0.clone(), value: header.1.clone() });
                    headers.push(header);
                }
                DynamicTableEntry::Method(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if method.is_some() {
                        return Err(DecompressionError::DuplicateMethod);
                    }
                    dynamic_table.insert(DynamicTableEntry::Method(val.clone()));
                    method = Some(val);
                }
                DynamicTableEntry::Path(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if path.is_some() {
                        return Err(DecompressionError::DuplicatePath);
                    }
                    validate_path(val.as_ref())?;
                    dynamic_table.insert(DynamicTableEntry::Path(val.clone()));
                    path = Some(val.as_ref().to_string());
                }
                DynamicTableEntry::Scheme(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if scheme.is_some() {
                        return Err(DecompressionError::DuplicateScheme);
                    }
                    dynamic_table.insert(DynamicTableEntry::Scheme(val.clone()));
                    scheme = Some(val);
                }
            }
            continue;
        }

        // 6.3. Dynamic Table Size Update
        if first_octet & 0x20 == 0x20 {
            let Some(max_size) = request.read_integer(first_octet & 0x1F, 5) else {
                return Err(DecompressionError::UnexpectedEndOfFile);
            };

            if !is_first {
                return Err(DecompressionError::DynamicTableUpdateNotFirst);
            }

            if dynamic_table.max_size < max_size {
                return Err(DecompressionError::DynamicTableUpdateTooLarge);
            }

            dynamic_table.size_update(max_size);
            continue;
        }

        // 6.2.3. Literal Header Field Never Indexed
        if first_octet & 0x10 == 0x10 {
            if first_octet == 0x10 {
                // Literal Header Field Never Indexed — New Name
                let name = request.read_string().ok_or(DecompressionError::UnexpectedEndOfFile)?;
                validate_header_name(&name)?;

                let value = request.read_string().ok_or(DecompressionError::UnexpectedEndOfFile)?;
                validate_header_value(&value)?;

                let header = (HeaderName::from(name), HeaderValue::from(value));
                validate_header(&header)?;
                headers.push(header);
                continue;
            }

            // Literal Header Field Never Indexed — Indexed Name
            let Some(index) = request.read_integer(first_octet & 0x0F, 4) else {
                return Err(DecompressionError::UnexpectedEndOfFile);
            };

            let value = request.read_string().ok_or(DecompressionError::UnexpectedEndOfFile)?;
            validate_header_value(&value)?;

            let entry = dynamic_table.get(index, Some(value))?;
            match entry {
                DynamicTableEntry::Authority(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if authority.is_some() {
                        return Err(DecompressionError::DuplicateAuthority);
                    }
                    authority = Some(val);
                },
                DynamicTableEntry::Header { name, value } => {
                    let header = (name, value);
                    validate_header(&header)?;
                    headers.push(header);
                }
                DynamicTableEntry::Method(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if method.is_some() {
                        return Err(DecompressionError::DuplicateMethod);
                    }
                    method = Some(val);
                }
                DynamicTableEntry::Path(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    validate_path(val.as_ref())?;
                    if path.is_some() {
                        return Err(DecompressionError::DuplicatePath);
                    }
                    path = Some(val.as_ref().to_string());
                }
                DynamicTableEntry::Scheme(val) => {
                    if !headers.is_empty() {
                        return Err(DecompressionError::PseudoAfterRegularFields);
                    }
                    if scheme.is_some() {
                        return Err(DecompressionError::DuplicateScheme);
                    }
                    scheme = Some(val);
                }
            }
            continue;
        }

        // 6.2.2. Literal Header Field without Indexing
        debug_assert!(first_octet < 0x10);

        // Literal Header Field without Indexing — New Name
        if first_octet == 0 {
            let name = request.read_string().ok_or(DecompressionError::UnexpectedEndOfFile)?;
            validate_header_name(&name)?;

            let value = request.read_string().ok_or(DecompressionError::UnexpectedEndOfFile)?;
            validate_header_value(&value)?;

            let header = (HeaderName::from(name), HeaderValue::from(value));
            validate_header(&header)?;

            headers.push(header);
            continue;
        }

        // Literal Header Field without Indexing — Indexed Name
        let Some(index) = request.read_integer(first_octet & 0x0F, 4) else {
            return Err(DecompressionError::UnexpectedEndOfFile);
        };

        let value = request.read_string().ok_or(DecompressionError::UnexpectedEndOfFile)?;
        validate_header_value(&value)?;

        let entry = dynamic_table.get(index, Some(value))?;
        match entry {
            DynamicTableEntry::Authority(val) => {
                if !headers.is_empty() {
                    return Err(DecompressionError::PseudoAfterRegularFields);
                }
                if authority.is_some() {
                    return Err(DecompressionError::DuplicateAuthority);
                }
                authority = Some(val);
            },
            DynamicTableEntry::Header { name, value } => {
                let header = (name, value);
                validate_header(&header)?;
                headers.push(header);
            }
            DynamicTableEntry::Method(val) => {
                if !headers.is_empty() {
                    return Err(DecompressionError::PseudoAfterRegularFields);
                }
                if method.is_some() {
                    return Err(DecompressionError::DuplicateMethod);
                }
                method = Some(val);
            }
            DynamicTableEntry::Path(val) => {
                if !headers.is_empty() {
                    return Err(DecompressionError::PseudoAfterRegularFields);
                }
                validate_path(val.as_ref())?;
                if path.is_some() {
                    return Err(DecompressionError::DuplicatePath);
                }
                path = Some(val.as_ref().to_string());
            }
            DynamicTableEntry::Scheme(val) => {
                if !headers.is_empty() {
                    return Err(DecompressionError::PseudoAfterRegularFields);
                }
                if scheme.is_some() {
                    return Err(DecompressionError::DuplicateScheme);
                }
                dynamic_table.insert(DynamicTableEntry::Scheme(val.clone()));
                scheme = Some(val);
            }
        }
    }

    let Some(path) = path else {
        return Err(DecompressionError::NoPath);
    };

    let Some(method) = method else {
        return Err(DecompressionError::NoMethod);
    };

    if scheme.is_none() && method != Method::Connect {
        return Err(DecompressionError::NoScheme);
    }

    Ok(Request {
        method,
        target: RequestTarget::Origin { path, query: String::new() },
        version: HttpVersion::Http2,
        headers: HeaderMap::new_with_vec(headers),
        body: None
    })
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_cursor: usize,
    bit_cursor: u8,
    total_bits: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8], max_bits: Option<usize>) -> Self {
        BitReader {
            data,
            byte_cursor: 0,
            bit_cursor: match max_bits {
                Some(x) if x < 8 => x as _,
                _ => 7,
            },
            total_bits: match max_bits {
                Some(max_bits) => {
                    assert!(max_bits <= data.len() * 8);
                    max_bits
                }
                None => data.len() * 8,
            }
        }
    }

    pub fn bits_left(&self) -> usize {
        let bit_position = self.bit_position();

        if bit_position >= self.total_bits {
            0
        } else {
            self.total_bits - bit_position
        }
    }

    fn bit_position(&self) -> usize {
        let bits_in_current_byte = if self.byte_cursor == self.data.len() - 1 && self.total_bits % 8 != 0 {
            self.total_bits % 8
        } else {
            7
        };
        (self.byte_cursor * 8 + bits_in_current_byte) - self.bit_cursor as usize
    }
}

impl<'a> Iterator for BitReader<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        let bits_left = self.bits_left();
        if bits_left == 0 {
            return None;
        }

        let state = ((self.data[self.byte_cursor] >> self.bit_cursor) & 0b1) == 1;

        let bits_left = bits_left - 1;
        if self.bit_cursor == 0 {
            self.byte_cursor += 1;
            if bits_left < 8 {
                self.bit_cursor = bits_left as _;
            } else {
                self.bit_cursor = 7;
            }
        } else {
            self.bit_cursor -= 1;
        }

        Some(state)
    }
}

struct BitReader3 {
    data: u32,
    bits: u8,
    bit_position: u8,
}

impl BitReader3 {
    pub fn new(data: u32, bits: u8) -> Self {
        assert!(bits <= 32);
        Self { data, bits, bit_position: 0 }
    }
}

impl Iterator for BitReader3 {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        if self.bit_position == self.bits {
            return None;
        }

        let state = (self.data >> (self.bits - 1 - self.bit_position)) & 1 == 1;
        self.bit_position += 1;
        Some(state)
    }
}

/// A HPACK bit writer, with '1' padding if necessary.
struct BitWriter<'a> {
    data: &'a mut dyn Write,
    current_byte: u8,
    bit_position: u8,
}

impl<'a> BitWriter<'a> {
    pub fn new(data: &'a mut dyn Write) -> Self {
        Self {
            data,
            current_byte: 0,
            bit_position: 7,
        }
    }

    /// Write a bit to the stream, only failing when the propagation of bytes
    /// failed.
    pub fn push(&mut self, value: bool) -> Result<(), std::io::Error> {
        self.current_byte |= (value as u8) << self.bit_position;
        if self.bit_position == 0 {
            self.data.write_all(&[self.current_byte])?;
            self.current_byte = 0;
            self.bit_position = 7;
        } else {
            self.bit_position -= 1;
        }

        Ok(())
    }
}

impl<'a> Drop for BitWriter<'a> {
    fn drop(&mut self) {
        if self.bit_position != 7 {
            // Finish the byte by padding the leftover bits.
            let finish_byte = self.current_byte | (2_u8.pow(self.bit_position as u32 + 1) - 1);
            self.data.write_all(&[finish_byte]).unwrap();
        }
    }
}

pub(super) fn decode_huffman(input: &[u8]) -> Option<String> {
    let mut output = Vec::new();

    let mut current_number = 0;
    let mut bit_length = 0;
    for bit in BitReader::new(input, None) {
        if bit_length == 255 {
            debug_assert!(false, "Shouldn't practically be possible");
            return None;
        }

        bit_length += 1;
        current_number <<= 1;
        if bit {
            current_number |= 1;
        }

        if let Some(symbols_for_bit_length) = HUFFMAN_TREE.table.get(&bit_length) {
            if let Some(symbol) = symbols_for_bit_length.get(&current_number) {
                match symbol {
                    HuffmanValue::EndOfStream => {
                        return None;
                    }
                    HuffmanValue::Symbol(symbol) => output.push(*symbol),
                }
                bit_length = 0;
                current_number = 0;
            }
        }
    }

    if bit_length > 7 {
        return None;
    }

    if bit_length != 0 {
        let correct_padding = 2_u32.pow(bit_length as _) - 1;
        if correct_padding != current_number {
            return None;
        }
    }

    Some(String::from_utf8(output).unwrap()) // TODO propagate errors correcty
}

/// Validate the header names for applicability, governed by
/// [RFC 9113 Section 8.2.2](https://httpwg.org/specs/rfc9113.html#rfc.section.8.2.2)
///
/// # References
/// * [RFC 9113 Section 8.2.2](https://httpwg.org/specs/rfc9113.html#rfc.section.8.2.2)
fn validate_header(header: &(HeaderName, HeaderValue)) -> Result<(), DecompressionError> {
    let (name, value) = header;

    // 8.2.2. Connection-Specific Header Fields
    match name {
        HeaderName::Connection | HeaderName::KeepAlive |
        HeaderName::ProxyConnection | HeaderName::TransferEncoding |
        HeaderName::Upgrade => return Err(DecompressionError::ConnectionSpecificHeaderField),
        HeaderName::TE => {
            if value.as_str_no_convert() != Some("trailers") {
                return Err(DecompressionError::TeHeaderNotTrailers);
            }
        }
        _ => ()
    }

    Ok(())
}

/// Validate the header names for non-indexed headers, governed by
/// [RFC 9113 Section 8.2](https://httpwg.org/specs/rfc9113.html#rfc.section.8.2)
///
/// # References
/// * [RFC 9113 Section 8.2](https://httpwg.org/specs/rfc9113.html#rfc.section.8.2)
fn validate_header_name(name: &str) -> Result<(), DecompressionError> {
    if name.is_empty() {
        return Err(DecompressionError::FieldNameEmpty);
    }

    if name.starts_with(':') && name != ":protocol" {
        return Err(DecompressionError::FieldNameStartWithColonNonPseudoField);
    }

    for c in name.bytes() {
        match c {
            0x00..=0x19 => return Err(DecompressionError::FieldNameInvalidNonVisibleAscii),
            0x20 => return Err(DecompressionError::FieldNameInvalidAsciiSpace),
            0x41..=0x5a => return Err(DecompressionError::FieldNameInvalidUppercase),
            0x7f..=0xff => return Err(DecompressionError::FieldNameExtendedAsciiUnicode),
            _ => (),
        }
    }

    Ok(())
}

/// Validate the header values for non-indexed headers, governed by
/// [RFC 9113 Section 8.2](https://httpwg.org/specs/rfc9113.html#rfc.section.8.2)
///
/// # References
/// * [RFC 9113 Section 8.2](https://httpwg.org/specs/rfc9113.html#rfc.section.8.2)
fn validate_header_value(value: &str) -> Result<(), DecompressionError> {
    if value.starts_with(' ') || value.starts_with('\t') {
        return Err(DecompressionError::FieldValueStartsWithWhitespace);
    }

    if value.ends_with(' ') || value.ends_with('\t') {
        return Err(DecompressionError::FieldValueEndsWithWhitespace);
    }

    for c in value.bytes() {
        match c {
            0x00 => return Err(DecompressionError::FieldValueContainsNul),
            0x0a => return Err(DecompressionError::FieldValueContainsLineFeed),
            0x0d => return Err(DecompressionError::FieldValueContainsCarriageReturn),
            _ => (),
        }
    }

    Ok(())
}

/// # References
/// * [RFC 9113 - Section 8.3.1](https://httpwg.org/specs/rfc9113.html#rfc.section.8.3.1)
fn validate_path(value: &str) -> Result<(), DecompressionError> {
    if value.is_empty() {
        return Err(DecompressionError::EmptyPath);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use servente_http::{RequestTarget, HttpVersion};
    use crate::{BinaryRequest, StreamId};

    use super::*;

    #[test]
    fn test_static_table() {
        assert_eq!(STATIC_TABLE.len(), 62);
        assert_eq!(STATIC_TABLE[0], StaticTableEntry::Illegal);
        assert_eq!(STATIC_TABLE[1], StaticTableEntry::Authority);
        assert_eq!(STATIC_TABLE[8], StaticTableEntry::Status(StatusCode::Ok));
        assert_eq!(STATIC_TABLE[14], StaticTableEntry::Status(StatusCode::InternalServerError));
        assert_eq!(STATIC_TABLE[15], StaticTableEntry::Header(HeaderName::AcceptCharset));
        assert_eq!(STATIC_TABLE[61], StaticTableEntry::Header(HeaderName::WwwAuthenticate));
    }

    #[test]
    fn test_dynamic_table_empty() {
        let table = DynamicTable::new(0);
        assert_eq!(table.get(0, None), Err(DynamicTableLookupError::InvalidIndex));
        assert_eq!(table.get(1, None), Err(DynamicTableLookupError::PseudoHeaderWithoutValue));

        assert_eq!(table.get(2, None), Ok(DynamicTableEntry::Method(Method::Get)));
        assert_eq!(table.get(2, Some(String::from("OPTIONS"))), Ok(DynamicTableEntry::Method(Method::Options)));
        assert_eq!(table.get(3, None), Ok(DynamicTableEntry::Method(Method::Post)));
        assert_eq!(table.get(3, Some(String::from("DELETE"))), Ok(DynamicTableEntry::Method(Method::Delete)));

        assert_eq!(table.get(4, None), Ok(DynamicTableEntry::Path(StaticOrSharedString::Static("/"))));
        assert_eq!(table.get(5, None), Ok(DynamicTableEntry::Path(StaticOrSharedString::Static("/index.html"))));
        assert_eq!(table.get(4, Some(String::from("/test.png"))), Ok(DynamicTableEntry::Path(StaticOrSharedString::Shared(Arc::from("/test.png")))));

        for index in 8..15 {
            assert_eq!(table.get(index, None), Err(DynamicTableLookupError::PseudoHeaderStatus));
        }

        assert_eq!(table.get(STATIC_TABLE.len(), None), Err(DynamicTableLookupError::OutOfBounds));
        assert_eq!(table.get(usize::MAX, None), Err(DynamicTableLookupError::OutOfBounds));
    }

    #[test]
    fn test_dynamic_table_some() {
        let mut table = DynamicTable::new(47 + 47);
        table.insert(DynamicTableEntry::Method(Method::Connect));
        table.insert(DynamicTableEntry::Header { name: HeaderName::SecChUaMobile, value: "?0".into() });

        let first = table.get(STATIC_TABLE.len(), None).unwrap();
        if let DynamicTableEntry::Header { name, value } = first {
            assert_eq!(name, HeaderName::SecChUaMobile);
            assert_eq!(value.as_str_no_convert(), Some("?0"));
        } else {
            panic!("invalid type: {:#?}", first);
        }

        assert_eq!(table.get(STATIC_TABLE.len() + 1, None), Ok(DynamicTableEntry::Method(Method::Connect)));
    }

    #[test]
    fn test_dynamic_table_eviction() {
        let mut table = DynamicTable::new(44 * 2);
        table.insert(DynamicTableEntry::Method(Method::Post));
        table.insert(DynamicTableEntry::Method(Method::Pri));
        table.insert(DynamicTableEntry::Method(Method::Put));

        assert_eq!(table.get(STATIC_TABLE.len(), None), Ok(DynamicTableEntry::Method(Method::Put)));
        assert_eq!(table.get(STATIC_TABLE.len() + 1, None), Ok(DynamicTableEntry::Method(Method::Pri)));
    }

    #[test]
    fn test_dynamic_table_insert_max_0() {
        let mut table = DynamicTable::new(0);
        assert_eq!(table.get(STATIC_TABLE.len(), None), Err(DynamicTableLookupError::OutOfBounds));

        table.insert(DynamicTableEntry::Authority("localhost".into()));

        assert_eq!(table.get(STATIC_TABLE.len(), None), Err(DynamicTableLookupError::OutOfBounds));
    }

    #[test]
    fn test_calculate_size() {
        assert_eq!(DynamicTableEntry::Authority("www.example.com".into()).calculate_size(), 57);
        assert_eq!(DynamicTableEntry::Authority("".into()).calculate_size(), 42);
        assert_eq!(DynamicTableEntry::Header { name: HeaderName::Other(String::new()), value: "".into() }.calculate_size(), 32);
        assert_eq!(DynamicTableEntry::Header { name: HeaderName::Other(String::from("custom-key")), value: "custom-value".into() }.calculate_size(), 54);
        assert_eq!(DynamicTableEntry::Header { name: HeaderName::Connection, value: "close".into() }.calculate_size(), 47);
        assert_eq!(DynamicTableEntry::Header { name: HeaderName::CacheControl, value: "no-cache".into() }.calculate_size(), 53);
        assert_eq!(DynamicTableEntry::Header { name: HeaderName::SetCookie, value: "foo=ASDJKHQKBZXOQWEOPIUAXQWEOIU; max-age=3600; version=1".into() }.calculate_size(), 98);
        assert_eq!(DynamicTableEntry::Header { name: HeaderName::ContentEncoding, value: "gzip".into() }.calculate_size(), 52);
        assert_eq!(DynamicTableEntry::Header { name: HeaderName::Date, value: "Mon, 21 Oct 2013 20:13:22 GMT".into() }.calculate_size(), 65);
        assert_eq!(DynamicTableEntry::Scheme("https".into()).calculate_size(), 44);

        assert_eq!(DynamicTableEntry::Header { name: HeaderName::Location, value: "https://www.example.com".into() }.calculate_size(), 63);
        assert_eq!(DynamicTableEntry::Header { name: HeaderName::CacheControl, value: "private".into() }.calculate_size(), 52);
        assert_eq!(DynamicTableEntry::Method(Method::BaselineControl).calculate_size(), ":method".len() + "BASELINE-CONTROL".len() + 32);
    }

    use super::BitWriter;

    #[test]
    fn test_bitwriter_nop() {
        let mut data = Vec::new();
        let writer = BitWriter::new(&mut data);
        drop(writer);
        assert!(data.is_empty());
    }

    #[test]
    fn test_bitwrite_zeros() {
        let mut data = Vec::new();
        {
            let mut writer = BitWriter::new(&mut data);
            for _ in 0..8 {
                writer.push(false).unwrap();
            }
        }
        assert_eq!(data.len(), 1);
        assert_eq!(data.first(), Some(&0));
    }

    #[test]
    fn test_bitwrite_ones() {
        let mut data = Vec::new();
        {
            let mut writer = BitWriter::new(&mut data);
            for _ in 0..8 {
                writer.push(true).unwrap();
            }
        }
        assert_eq!(data.len(), 1);
        assert_eq!(data.first(), Some(&255));
    }

    #[rstest]
    #[case(&[false], 0b0111_1111_u8)]
    #[case(&[true], 0b1111_1111_u8)]
    #[case(&[true, false], 0b1011_1111_u8)]
    #[case(&[false, false, false, false], 0b0000_1111_u8)]
    #[case(&[false, false, false, false, true, true, true, true], 0b0000_1111_u8)]
    #[case(&[true, true, true, true, false, false, false, false], 0b1111_0000_u8)]
    #[case(&[true, true, true, true, true, true, true, true], 0b1111_1111_u8)]
    #[case(&[false, false, false, false, false, false, false, false], 0b0000_0000_u8)]
    #[test]
    fn test_bitwrite_cases_one(#[case] bits: &[bool], #[case] expected_value: u8) {
        let mut data = Vec::new();
        {
            let mut writer = BitWriter::new(&mut data);
            for b in bits {
                writer.push(*b).unwrap();
            }
        }
        assert_eq!(data.len(), 1);
        assert_eq!(data.first(), Some(&expected_value));
    }

    #[rstest]
    #[case(&[], &[])]
    #[case(&[false, false, false, false, false, false, false, false, false], &[0, 0b0111_1111])]
    #[case(&[false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false], &[0, 0])]
    #[test]
    fn test_bitwrite_cases_multiple(#[case] bits: &[bool], #[case] expected: &[u8]) {
        let mut data = Vec::new();
        {
            let mut writer = BitWriter::new(&mut data);
            for b in bits {
                writer.push(*b).unwrap();
            }
        }
        assert_eq!(data.as_slice(), expected);
    }

    #[rstest]
    #[case(&[0xf1, 0xe3, 0xc2, 0xe5, 0xf2, 0x3a, 0x6b, 0xa0, 0xab, 0x90, 0xf4, 0xff], Some("www.example.com"))]
    #[case(&[0xa8, 0xeb, 0x10, 0x64, 0x9c, 0xbf], Some("no-cache"))]
    #[test]
    fn test_decode_huffman(#[case] input: &[u8], #[case] expected: Option<&str>) {
        assert_eq!(decode_huffman(input).as_deref(), expected);
    }

    // Encode into Huffman helper for other tests
    fn encode_huffman(input: &str) -> Vec<u8> {
        let mut data = Vec::new();
        data.write_hpack_string_huffman(input).unwrap();
        data
    }

    #[rstest]
    #[case("no-cache", &[0x86, 0xa8, 0xeb, 0x10, 0x64, 0x9c, 0xbf])]
    #[case("n", &[0x81, 0b10101011])]
    #[test]
    fn test_encode_huffman(#[case] input: &str, #[case] expected: &[u8]) {
        let encoded = encode_huffman(input);
        for byte in &encoded {
            println!("[TEST] Encoded: hex {byte:#x} \tor dec {byte}\t or bin {byte:#b}");
        }
        assert_eq!(&encoded, expected, "Incorrect encode: {encoded:x?}  expected: {expected:x?}");
    }

    #[rstest]
    #[case("hello")]
    #[case("text/html; charset=utf-8")]
    #[case("'")]
    #[case("default-src 'self'; upgrade-insecure-requests; style-src-elem 'self' 'unsafe-inline'")]
    #[case("Thu, 01 Jan 1970 00:00:00 GMT")]
    #[case("cache-control: public, max-age=600")]
    #[case("welcome-en")]
    #[test]
    fn test_encode_with_decode(#[case] input: &str) {
        let encoded = encode_huffman(input);
        println!("Encoded({}): {:?}", encoded.len(), encoded);
        let decoded = decode_huffman(&encoded[1..]);
        assert_eq!(decoded.as_deref(), Some(input));
    }

    #[rstest]
    #[case("www.example.com", &[0x8c, 0xf1, 0xe3, 0xc2, 0xe5, 0xf2, 0x3a, 0x6b, 0xa0, 0xab, 0x90, 0xf4, 0xff])]
    #[test]
    fn test_binary_request_read_string(#[case] expected: &str, #[case] input: &[u8]) {
        let mut binary_request = BinaryRequest {
            stream_id: StreamId(1),
            headers: vec![Vec::from(input)],
            data: Vec::new(),
            cursor: 0,
        };
        assert_eq!(binary_request.read_string().as_deref(), Some(expected));
    }

    /// A test for the HPACK example C.4.1. First Request
    ///
    /// ```text
    /// :method: GET
    /// :scheme: http
    /// :path: /
    /// :authority: www.example.com
    /// ```
    #[tokio::test]
    async fn test_decode_hpack_example_4_1() {
        let data = vec![
            // static table entry #2
            // :method: GET
            0x82,
            // static table entry #6
            // :scheme: http
            0x86,
            // static table entry #4
            // :path: /
            0x84,

            // literal header field with incremental indexing
            // indexed name: static table entry #1
            // :authority
            0x41,

            // Literal value of length 12
            // binary: 10001100
            //         H   84
            0x8c,
            // "www.example.com" Huffman encoded
            0xf1, 0xe3, 0xc2, 0xe5, 0xf2, 0x3a, 0x6b, 0xa0, 0xab, 0x90, 0xf4, 0xff
        ];
        let dynamic_table = Arc::new(Mutex::new(DynamicTable::new(4096)));
        let request = BinaryRequest {
            stream_id: StreamId(1),
            headers: vec![data],
            data: Vec::new(),
            cursor: 0,
        };

        let request = decode_hpack(request, dynamic_table).await.unwrap();
        assert_eq!(request.method, Method::Get);
        assert_eq!(request.target, RequestTarget::Origin { path: "/".to_string(), query: String::new() });
        assert_eq!(request.headers.iter().next(), None);
    }

    #[tokio::test]
    async fn test_decode_hpack_curl_1() {
        let data = vec![
            0x82, 0x84, 0x87, 0x41, 0x8a, 0xa0, 0xe4, 0x1d, 0x13, 0x9d, 0x09,
            0xb8, 0xf0, 0x1e, 0x07, 0x7a, 0x88, 0x25, 0xb6, 0x50, 0xc3, 0xab,
            0xbc, 0xea, 0xe0, 0x53, 0x03, 0x2a, 0x2f, 0x2a
        ];
        let dynamic_table = Arc::new(Mutex::new(DynamicTable::new(4096)));
        let request = BinaryRequest {
            stream_id: StreamId(1),
            headers: vec![data],
            data: Vec::new(),
            cursor: 0,
        };

        let request = decode_hpack(request, dynamic_table).await.unwrap();
        assert_eq!(request.method, Method::Get);
        assert_eq!(request.target, RequestTarget::Origin { path: "/".to_string(), query: String::new() });
        assert_eq!(request.version, HttpVersion::Http2);
        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.headers.get(&HeaderName::UserAgent), Some(&HeaderValue::from(String::from("curl/7.87.0"))));
        assert_eq!(request.headers.get(&HeaderName::Accept), Some(&HeaderValue::from(String::from("*/*"))));
    }

    #[rstest]
    #[case(37, 4, 0, &[0x0F, 0x16])]
    #[case(10, 4, 0, &[0b0000_1010])]
    fn test_write_hpack_number(#[case] input: usize, #[case] n: u8, #[case] prefix: u8, #[case] expected: &[u8]) {
        let mut buf = Vec::new();
        buf.write_hpack_number(input, n, prefix).unwrap();
        assert_eq!(buf.as_slice(), expected);
    }

    #[rstest]
    #[case((HeaderName::Connection, "keep-alive".into()), Err(DecompressionError::ConnectionSpecificHeaderField))]
    #[case((HeaderName::Connection, "close".into()), Err(DecompressionError::ConnectionSpecificHeaderField))]
    #[case((HeaderName::Connection, "some-non-standard-token".into()), Err(DecompressionError::ConnectionSpecificHeaderField))]
    #[case((HeaderName::ProxyConnection, "close".into()), Err(DecompressionError::ConnectionSpecificHeaderField))]
    #[case((HeaderName::ProxyConnection, "keep-alive".into()), Err(DecompressionError::ConnectionSpecificHeaderField))]
    #[case((HeaderName::ProxyConnection, "some-non-standard-token".into()), Err(DecompressionError::ConnectionSpecificHeaderField))]
    #[case((HeaderName::TE, "trailers".into()), Ok(()))]
    #[case((HeaderName::TE, "some-non-standard-token".into()), Err(DecompressionError::TeHeaderNotTrailers))]
    #[case((HeaderName::TE, "compress".into()), Err(DecompressionError::TeHeaderNotTrailers))]
    #[case((HeaderName::TE, "deflate".into()), Err(DecompressionError::TeHeaderNotTrailers))]
    #[case((HeaderName::TE, "trailers, compress".into()), Err(DecompressionError::TeHeaderNotTrailers))]
    #[case((HeaderName::TE, "compress, trailers".into()), Err(DecompressionError::TeHeaderNotTrailers))]
    #[case((HeaderName::TE, "gzip".into()), Err(DecompressionError::TeHeaderNotTrailers))]
    #[test]
    fn test_validate_header(#[case] header: (HeaderName, HeaderValue), #[case] expected_result: Result<(), DecompressionError>) {
        assert_eq!(validate_header(&header), expected_result);
    }
}
