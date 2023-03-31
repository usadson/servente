// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

#[derive(Debug)]
#[repr(u32)]
#[allow(dead_code)]
pub enum ErrorCode {
    H3DatagramError = 0x33,
    H3NoError,
    H3GeneralProtocolError,
    H3InternalError,
    H3StreamCreationError,
    H3ClosedStreamCritical,
    H3FrameUnexpected,
    H3FrameError,
    H3ExcessiveLoad,
    H3IdError,
    H3SettingsError,
    H3MissingSettings,
    H3RequestRejected,
    H3RequestCancelled,
    H3RequestIncomplete,
    H3MessageError,
    H3ConnectError,
    H3VersionFallback,
    QpackDecompressionFailed,
    QpackEncoderStreamError,
    QpackDecoderStreamError,
}

#[derive(Debug)]
pub enum ReadError {
    ConnectionError(ErrorCode),
    FinishedEarly,
    IoError(std::io::Error),
    NonAsciiOrigin,
    NonAsciiPriorityUpdate,
    ReadError(quinn::ReadError),
}

impl ReadError {
    pub fn is_stream_closed(&self) -> bool {
        match self {
            Self::ReadError(error) => match error {
                quinn::ReadError::Reset(_) | quinn::ReadError::UnknownStream => {
                    true
                }
                _ => false
            }
            Self::IoError(error) => error.kind() == std::io::ErrorKind::UnexpectedEof,
            _ => false,
        }
    }
}

impl From<quinn::ReadError> for ReadError {
    fn from(value: quinn::ReadError) -> Self {
        Self::ReadError(value)
    }
}

impl From<std::io::Error> for ReadError {
    fn from(value: std::io::Error) -> Self {
        // Tokio abstractions (AsyncRead) of `quinn` convert the error to an
        // [`std::io::Error`], and to make the errors more easy to detect we
        // try to convert them back.
        if let Some(e) = value.get_ref() {
            if let Some(e) = e.downcast_ref::<quinn::ReadError>() {
                return Self::ReadError(e.clone());
            }
        }
        println!("Error: {value:?}");
        Self::IoError(value)
    }
}

impl From<quinn::ReadExactError> for ReadError {
    fn from(value: quinn::ReadExactError) -> Self {
        match value {
            quinn::ReadExactError::FinishedEarly => Self::FinishedEarly,
            quinn::ReadExactError::ReadError(err) => err.into(),
        }
    }
}
