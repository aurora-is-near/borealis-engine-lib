//! Messages on the Borealis bus are serialized using CBOR
//!
//! (see <https://github.com/aurora-is-near/borealis-spec#message-format>).
//! However, CBOR is a very loose format in the sense that there are multiple ways to serialize
//! the same data (for example serializing a struct as an array or a map).
//! Therefore, we choose to not derive `serde::Serialize` and `serde::Deserialize` as we must have
//! complete control over the details of how each structure is encoded in CBOR.
//! To that end, this module defines the `ToCbor` and `FromCbor` traits which must be implemented
//! for data types that will be sent over the wire.

use crate::bus_message::error::UnknownMessageType;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

pub trait ToCbor: Sized {
    fn to_cbor(self) -> Result<serde_cbor::Value, EncodeError>;

    fn serialize(self) -> Result<Vec<u8>, EncodeError> {
        let value = self.to_cbor()?;
        let bytes = serde_cbor::to_vec(&value)?;
        Ok(bytes)
    }
}

pub trait FromCbor: Sized {
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CborType {
    Array,
    Bool,
    Bytes,
    Float,
    Integer,
    Map,
    Missing,
    Text,
    Tagged(Box<Self>),
}

impl CborType {
    #[must_use]
    pub fn of_value(v: &serde_cbor::Value) -> Self {
        match v {
            serde_cbor::Value::Bool(_) => Self::Bool,
            serde_cbor::Value::Integer(_) => Self::Integer,
            serde_cbor::Value::Float(_) => Self::Float,
            serde_cbor::Value::Bytes(_) => Self::Bytes,
            serde_cbor::Value::Text(_) => Self::Text,
            serde_cbor::Value::Array(_) => Self::Array,
            serde_cbor::Value::Map(_) => Self::Map,
            serde_cbor::Value::Tag(_, inner) => Self::Tagged(Box::new(Self::of_value(inner))),
            _ => Self::Missing,
        }
    }
}

pub fn from_cbor_or_default<T: FromCbor + Default>(
    input: serde_cbor::Value,
) -> Result<T, DecodeError> {
    match T::from_cbor(input) {
        Ok(t) => Ok(t),
        Err(DecodeError::TypeMismatch { expected, found }) => {
            if found == CborType::Missing {
                Ok(T::default())
            } else {
                Err(DecodeError::TypeMismatch { expected, found })
            }
        }
        Err(e) => Err(e),
    }
}

pub fn get_byte_arr<const N: usize>(value: &serde_cbor::Value) -> Result<[u8; N], DecodeError> {
    let bytes = get_bytes(value)?;
    if bytes.len() != N {
        return Err(DecodeError::InvalidBytesLength {
            expected: N,
            found: bytes.len(),
        });
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(bytes.as_ref());
    Ok(arr)
}

pub fn get_bytes(value: &serde_cbor::Value) -> Result<Cow<'_, [u8]>, DecodeError> {
    match value {
        serde_cbor::Value::Bytes(x) => Ok(Cow::Borrowed(x)),
        serde_cbor::Value::Array(x) => cbor_array_to_vec_u8(x).map(Cow::Owned),
        other => Err(DecodeError::TypeMismatch {
            expected: CborType::Bytes,
            found: CborType::of_value(other),
        }),
    }
}

pub fn get_u8(value: &serde_cbor::Value) -> Result<u8, DecodeError> {
    match value {
        serde_cbor::Value::Integer(x) => {
            let x = *x;
            if x < 0 || i128::from(u8::MAX) < x {
                return Err(DecodeError::InvalidU8(x));
            }

            u8::try_from(x).map_err(|_| DecodeError::InvalidU8(x))
        }
        other => Err(DecodeError::TypeMismatch {
            expected: CborType::Integer,
            found: CborType::of_value(other),
        }),
    }
}

pub fn get_u16(value: &serde_cbor::Value) -> Result<u16, DecodeError> {
    match value {
        serde_cbor::Value::Integer(x) => {
            let x = *x;
            if x < 0 || i128::from(u16::MAX) < x {
                return Err(DecodeError::InvalidU16(x));
            }

            u16::try_from(x).map_err(|_| DecodeError::InvalidU16(x))
        }
        other => Err(DecodeError::TypeMismatch {
            expected: CborType::Integer,
            found: CborType::of_value(other),
        }),
    }
}

pub fn get_u32(value: &serde_cbor::Value) -> Result<u32, DecodeError> {
    match value {
        serde_cbor::Value::Integer(x) => {
            let x = *x;
            if x < 0 || i128::from(u32::MAX) < x {
                return Err(DecodeError::InvalidU32(x));
            }

            u32::try_from(x).map_err(|_| DecodeError::InvalidU32(x))
        }
        other => Err(DecodeError::TypeMismatch {
            expected: CborType::Integer,
            found: CborType::of_value(other),
        }),
    }
}

impl FromCbor for u8 {
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        get_u8(&input)
    }
}

impl FromCbor for u16 {
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        get_u16(&input)
    }
}

impl FromCbor for u32 {
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        get_u32(&input)
    }
}

impl FromCbor for u64 {
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        get_u64(&input)
    }
}

impl<const N: usize> FromCbor for [u8; N] {
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        get_byte_arr(&input)
    }
}

impl FromCbor for Vec<u8> {
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        match input {
            serde_cbor::Value::Bytes(x) => Ok(x),
            serde_cbor::Value::Array(values) => cbor_array_to_vec_u8(&values),
            other => Err(DecodeError::TypeMismatch {
                expected: CborType::Bytes,
                found: CborType::of_value(&other),
            }),
        }
    }
}

impl FromCbor for String {
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        match input {
            serde_cbor::Value::Text(x) => Ok(x),
            other => Err(DecodeError::TypeMismatch {
                expected: CborType::Text,
                found: CborType::of_value(&other),
            }),
        }
    }
}

impl<K, V> FromCbor for BTreeMap<K, V>
where
    K: FromCbor + Ord,
    V: FromCbor + Default,
{
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        match input {
            serde_cbor::Value::Map(values) => {
                let mut result = Self::new();
                for (k, v) in values {
                    result.insert(K::from_cbor(k)?, from_cbor_or_default(v)?);
                }
                Ok(result)
            }
            other => Err(DecodeError::TypeMismatch {
                expected: CborType::Map,
                found: CborType::of_value(&other),
            }),
        }
    }
}

impl<A, B> FromCbor for (A, B)
where
    A: FromCbor,
    B: FromCbor,
{
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        match input {
            serde_cbor::Value::Array(mut values) => {
                if values.len() != 2 {
                    return Err(DecodeError::InvalidArrayLength {
                        expected: 2,
                        found: values.len(),
                    });
                }

                let (b, a) = (
                    B::from_cbor(values.pop().unwrap())?,
                    A::from_cbor(values.pop().unwrap())?,
                );
                Ok((a, b))
            }
            other => Err(DecodeError::TypeMismatch {
                expected: CborType::Array,
                found: CborType::of_value(&other),
            }),
        }
    }
}

impl<A, B, C> FromCbor for (A, B, C)
where
    A: FromCbor,
    B: FromCbor,
    C: FromCbor,
{
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        match input {
            serde_cbor::Value::Array(mut values) => {
                if values.len() != 3 {
                    return Err(DecodeError::InvalidArrayLength {
                        expected: 3,
                        found: values.len(),
                    });
                }

                let (c, b, a) = (
                    C::from_cbor(values.pop().unwrap())?,
                    B::from_cbor(values.pop().unwrap())?,
                    A::from_cbor(values.pop().unwrap())?,
                );
                Ok((a, b, c))
            }
            other => Err(DecodeError::TypeMismatch {
                expected: CborType::Array,
                found: CborType::of_value(&other),
            }),
        }
    }
}

impl<A, B, C, D> FromCbor for (A, B, C, D)
where
    A: FromCbor,
    B: FromCbor,
    C: FromCbor,
    D: FromCbor,
{
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        match input {
            serde_cbor::Value::Array(mut values) => {
                if values.len() != 4 {
                    return Err(DecodeError::InvalidArrayLength {
                        expected: 4,
                        found: values.len(),
                    });
                }

                let (d, c, b, a) = (
                    D::from_cbor(values.pop().unwrap())?,
                    C::from_cbor(values.pop().unwrap())?,
                    B::from_cbor(values.pop().unwrap())?,
                    A::from_cbor(values.pop().unwrap())?,
                );
                Ok((a, b, c, d))
            }
            other => Err(DecodeError::TypeMismatch {
                expected: CborType::Array,
                found: CborType::of_value(&other),
            }),
        }
    }
}

impl<A, B, C, D, E> FromCbor for (A, B, C, D, E)
where
    A: FromCbor,
    B: FromCbor,
    C: FromCbor,
    D: FromCbor,
    E: FromCbor,
{
    #[allow(clippy::many_single_char_names)]
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, DecodeError> {
        match input {
            serde_cbor::Value::Array(mut values) => {
                if values.len() != 5 {
                    return Err(DecodeError::InvalidArrayLength {
                        expected: 5,
                        found: values.len(),
                    });
                }

                let (e, d, c, b, a) = (
                    E::from_cbor(values.pop().unwrap())?,
                    D::from_cbor(values.pop().unwrap())?,
                    C::from_cbor(values.pop().unwrap())?,
                    B::from_cbor(values.pop().unwrap())?,
                    A::from_cbor(values.pop().unwrap())?,
                );

                Ok((a, b, c, d, e))
            }
            other => Err(DecodeError::TypeMismatch {
                expected: CborType::Array,
                found: CborType::of_value(&other),
            }),
        }
    }
}

pub fn get_u64(value: &serde_cbor::Value) -> Result<u64, DecodeError> {
    match value {
        serde_cbor::Value::Integer(x) => {
            let x = *x;
            if x < 0 || i128::from(u64::MAX) < x {
                return Err(DecodeError::InvalidU64(x));
            }

            u64::try_from(x).map_err(|_| DecodeError::InvalidU64(x))
        }
        other => Err(DecodeError::TypeMismatch {
            expected: CborType::Integer,
            found: CborType::of_value(other),
        }),
    }
}

fn cbor_array_to_vec_u8(values: &[serde_cbor::Value]) -> Result<Vec<u8>, DecodeError> {
    let mut result = Vec::with_capacity(values.len());
    for v in values {
        match get_u8(v) {
            Ok(n) => result.push(n),
            Err(DecodeError::TypeMismatch { .. }) => {
                return Err(DecodeError::TypeMismatch {
                    expected: CborType::Bytes,
                    found: CborType::Array,
                });
            }
            Err(e) => return Err(e),
        }
    }
    Ok(result)
}

#[derive(Debug)]
pub enum EncodeError {
    Io(std::io::Error),
    SerdeCbor(serde_cbor::Error),
    SerdeJson(serde_json::Error),
}

impl std::error::Error for EncodeError {}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::SerdeCbor(e) => e.fmt(f),
            Self::SerdeJson(e) => e.fmt(f),
        }
    }
}

impl From<std::io::Error> for EncodeError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl From<serde_cbor::Error> for EncodeError {
    fn from(source: serde_cbor::Error) -> Self {
        Self::SerdeCbor(source)
    }
}

impl From<serde_json::Error> for EncodeError {
    fn from(source: serde_json::Error) -> Self {
        Self::SerdeJson(source)
    }
}

impl From<EncodeError> for std::io::Error {
    fn from(e: EncodeError) -> Self {
        match e {
            EncodeError::Io(e) => e,
            EncodeError::SerdeJson(e) => e.into(),
            EncodeError::SerdeCbor(e) => Self::other(e),
        }
    }
}

#[derive(Debug)]
pub enum DecodeError {
    TypeMismatch { expected: CborType, found: CborType },
    InvalidArrayLength { expected: usize, found: usize },
    InvalidBytesLength { expected: usize, found: usize },
    InvalidU8(i128),
    InvalidU16(i128),
    InvalidU32(i128),
    InvalidU64(i128),
    MissingMapKey(serde_cbor::Value),
    UnknownMessageType(UnknownMessageType),
    SerdeCbor(serde_cbor::Error),
    SerdeJson(serde_json::Error),
    Io(std::io::Error),
}

impl std::error::Error for DecodeError {}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeMismatch { expected, found } => {
                write!(
                    f,
                    "CBOR type mismatch. Expected: {expected:?} Found {found:?}"
                )
            }
            Self::InvalidArrayLength { expected, found } => {
                write!(
                    f,
                    "Invalid CBOR array length. Expected: {expected:?} Found {found:?}"
                )
            }
            Self::InvalidBytesLength { expected, found } => {
                write!(
                    f,
                    "Invalid CBOR bytes length. Expected: {expected:?} Found {found:?}"
                )
            }
            Self::InvalidU8(x) => write!(f, "Invalid u8 value: {x:?}"),
            Self::InvalidU16(x) => write!(f, "Invalid u16 value: {x:?}"),
            Self::InvalidU32(x) => write!(f, "Invalid u32 value: {x:?}"),
            Self::InvalidU64(x) => write!(f, "Invalid u64 value: {x:?}"),
            Self::MissingMapKey(x) => write!(f, "CBOR map missing expected key: {x:?}"),
            Self::UnknownMessageType(e) => e.fmt(f),
            Self::SerdeCbor(e) => e.fmt(f),
            Self::SerdeJson(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
        }
    }
}

impl From<UnknownMessageType> for DecodeError {
    fn from(source: UnknownMessageType) -> Self {
        Self::UnknownMessageType(source)
    }
}

impl From<serde_cbor::Error> for DecodeError {
    fn from(source: serde_cbor::Error) -> Self {
        Self::SerdeCbor(source)
    }
}

impl From<serde_json::Error> for DecodeError {
    fn from(source: serde_json::Error) -> Self {
        Self::SerdeJson(source)
    }
}

impl From<std::io::Error> for DecodeError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}
