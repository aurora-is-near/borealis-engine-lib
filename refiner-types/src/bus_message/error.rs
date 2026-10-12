use crate::bus_message::bus_serde::{DecodeError, EncodeError};
use std::fmt;

#[derive(Debug)]
pub enum Error {
    CborEncoding(EncodeError),
    CborDecoding(DecodeError),
    Io(std::io::Error),
    MissingData,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CborEncoding(e) => e.fmt(f),
            Self::MissingData => write!(f, "Missing data in the payload"),
            Self::Io(io_error) => io_error.fmt(f),
            Self::CborDecoding(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<EncodeError> for Error {
    fn from(source: EncodeError) -> Self {
        Self::CborEncoding(source)
    }
}

impl From<serde_cbor::Error> for Error {
    fn from(source: serde_cbor::Error) -> Self {
        Self::CborEncoding(EncodeError::SerdeCbor(source))
    }
}

impl From<DecodeError> for Error {
    fn from(source: DecodeError) -> Self {
        Self::CborDecoding(source)
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

#[derive(Debug)]
pub struct UnknownMessageType(pub u16);

impl std::error::Error for UnknownMessageType {}

impl fmt::Display for UnknownMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown message type ID={}", self.0)
    }
}
