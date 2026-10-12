use bus_serde::{FromCbor, ToCbor};
use message_type::MessageType;
use payloads::Payload;
use rand::RngExt;
use serde::Deserialize;
use std::time::SystemTime;

pub mod bus_serde;
pub mod compression;
pub mod error;
pub mod message_type;
pub mod payloads;

/// Current version of Borealis Messages
pub const VERSION: u8 = 1;
/// Borealis epoch is equal to Bitcoin genesis, 2009-01-03T18:15:05Z
pub const BOREALIS_EPOCH: u64 = 1_231_006_505;

/// Borealis message format. See <https://github.com/aurora-is-near/borealis-spec#message-format>
/// for details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusMessage<T> {
    pub version: u8,
    pub envelope: Envelope,
    pub payload: T,
}

impl<T: Payload> BusMessage<T> {
    pub fn new<R: rand::Rng>(sequential_id: u64, rng: &mut R, payload: T) -> anyhow::Result<Self> {
        let now = SystemTime::UNIX_EPOCH.elapsed()?;
        let timestamp_sec = u32::try_from(now.as_secs() - BOREALIS_EPOCH)?;
        let timestamp_ms = (now.as_millis() % 1000) as u16;
        let unique_id = payload.unique_id().unwrap_or_else(|| rng.random());
        let envelope = Envelope {
            event_type: T::MESSAGE_TYPE,
            sequential_id,
            timestamp_s: timestamp_sec,
            timestamp_ms,
            unique_id,
        };

        Ok(Self {
            version: VERSION,
            envelope,
            payload,
        })
    }
}

impl<T: ToCbor> BusMessage<T> {
    pub fn serialize(self) -> Result<Vec<u8>, bus_serde::EncodeError> {
        let envelope_bytes = self.envelope.serialize()?;
        let payload_bytes = self.payload.serialize()?;
        let mut buf = Vec::with_capacity(1 + envelope_bytes.len() + payload_bytes.len());
        buf.push(self.version);
        buf.extend_from_slice(&envelope_bytes);
        buf.extend_from_slice(&payload_bytes);
        Ok(buf)
    }
}

impl<T: FromCbor> BusMessage<T> {
    pub fn deserialize(bytes: &[u8]) -> Result<Self, error::Error> {
        let (version, message) = bytes.split_first().ok_or(error::Error::MissingData)?;

        if message.is_empty() {
            return Err(error::Error::MissingData);
        }

        let mut deserializer = serde_cbor::Deserializer::from_slice(message);
        let envelope_value = serde_cbor::Value::deserialize(&mut deserializer)?;
        let envelope = Envelope::from_cbor(envelope_value)?;

        if deserializer.byte_offset() == message.len() {
            return Err(error::Error::MissingData);
        }

        let payload_value = serde_cbor::Value::deserialize(&mut deserializer)?;
        deserializer.end()?;
        let payload = T::from_cbor(payload_value)?;

        Ok(Self {
            version: *version,
            envelope,
            payload,
        })
    }
}

/// Borealis Message header. See <https://github.com/aurora-is-near/borealis-spec#message-envelope>
/// for details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub event_type: MessageType,
    pub sequential_id: u64,
    pub timestamp_s: u32,
    pub timestamp_ms: u16,
    pub unique_id: [u8; 16],
}

impl ToCbor for Envelope {
    fn to_cbor(self) -> Result<serde_cbor::Value, bus_serde::EncodeError> {
        let event_type = serde_cbor::Value::Integer(i128::from(self.event_type.to_u16()));
        let sequential_id = serde_cbor::Value::Integer(i128::from(self.sequential_id));
        let timestamp_sec = serde_cbor::Value::Integer(i128::from(self.timestamp_s));
        let timestamp_ms = serde_cbor::Value::Integer(i128::from(self.timestamp_ms));
        let unique_id = serde_cbor::Value::Bytes(self.unique_id.to_vec());

        Ok(serde_cbor::Value::Array(vec![
            event_type,
            sequential_id,
            timestamp_sec,
            timestamp_ms,
            unique_id,
        ]))
    }
}

impl FromCbor for Envelope {
    fn from_cbor(input: serde_cbor::Value) -> Result<Self, bus_serde::DecodeError> {
        let (event_type, sequential_id, timestamp_sec, timestamp_ms, unique_id) =
            <(u16, u64, u32, u16, [u8; 16])>::from_cbor(input)?;
        let event_type = MessageType::try_from_u16(event_type)?;
        Ok(Self {
            event_type,
            sequential_id,
            timestamp_s: timestamp_sec,
            timestamp_ms,
            unique_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Envelope;
    use super::bus_serde::{FromCbor, ToCbor};
    use super::message_type::MessageType;

    #[test]
    fn test_envelope_serde() {
        /* cbor.me annotation:
           85                                     # array(5)
              19 1020                             # unsigned(4128)
              1B 16EE1DAE0BC3AD58                 # unsigned(1652290746650439000)
              1A 191C4991                         # unsigned(421284241)
              19 028A                             # unsigned(650)
              50                                  # bytes(16)
                 E6637CE758EA9E804F5B49D9CCB55929 # "\xE6c|\xE7XꞀO[I\xD9̵Y)"
        */
        let expected_bytes = hex::decode(
            "851910201b16ee1dae0bc3ad581a191c499119028a50e6637ce758ea9e804f5b49d9ccb55929",
        )
        .unwrap();
        let expected_value = Envelope {
            event_type: MessageType::RpcRequest,
            sequential_id: 1_652_290_746_650_439_000,
            timestamp_s: 421_284_241,
            timestamp_ms: 650,
            unique_id: [
                230, 99, 124, 231, 88, 234, 158, 128, 79, 91, 73, 217, 204, 181, 89, 41,
            ],
        };
        let computed_bytes = expected_value.clone().serialize().unwrap();
        let computed_value =
            Envelope::from_cbor(serde_cbor::from_slice(&expected_bytes).unwrap()).unwrap();

        assert_eq!(expected_bytes, computed_bytes);
        assert_eq!(expected_value, computed_value);
    }
}
