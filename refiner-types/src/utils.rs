use aurora_engine_types::{H256, U256};
use sha3::Digest;

const U64_MAX: U256 = U256([u64::MAX, 0, 0, 0]);

pub mod u64_hex_serde {
    //! This module provides serde serialization for u64 numbers with hexadecimal encoding.
    //!
    //! It can be used with the field attribute `#[serde(with = "u64_hex_serde")]` on u64
    //! inside structs deriving serde Serialize and Deserialize traits.
    //! Note: if a number is larger than U256::MAX then the deserializing will fail with an error.
    //! If the number is less than or equal to U256::MAX, but larger than u64::MAX then it will
    //! be deserialized as 64::MAX. Allowing numbers up to U256::MAX preserves backwards compatibility
    //! with old data that included some (unused) field with default values equal to U256::MAX.

    use aurora_engine_types::U256;
    use serde::de::Error;

    pub fn serialize<S>(n: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_str = format!("{n:#x}");
        serializer.serialize_str(&hex_str)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str: String = serde::Deserialize::deserialize(deserializer)?;
        let hex_str = hex_str
            .strip_prefix("0x")
            .ok_or_else(|| D::Error::custom("Missing 0x Prefix"))?;
        let n = U256::from_str_radix(hex_str, 16).map_err(D::Error::custom)?;
        Ok(super::saturating_cast(n))
    }
}

pub mod u128_dec_serde {
    //! This module provides serde serialization for optional u128 numbers with base-10 strings.
    //!
    //! It can be used with the field attribute `#[serde(with = "u128_dec_serde")]` on u128 fields
    //! inside structs deriving serde Serialize and Deserialize traits.

    use serde::de::Error;

    pub fn serialize<S>(n: &Option<u128>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(n) = n {
            let dec_str = format!("{n}");
            serializer.serialize_some(&dec_str)
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u128>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let dec_str: Option<String> = serde::Deserialize::deserialize(deserializer)?;
        dec_str.map_or(Ok(None), |dec_str| {
            let n = dec_str.parse().map_err(D::Error::custom)?;
            Ok(Some(n))
        })
    }
}

/// Cast a U256 value down to u64; if the value is too large then return u64::MAX.
pub fn saturating_cast(x: U256) -> u64 {
    if x < U64_MAX { x.as_u64() } else { u64::MAX }
}

pub fn keccak256(input: &[u8]) -> H256 {
    let mut hasher = sha3::Keccak256::default();
    hasher.update(input);
    H256(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::{u64_hex_serde, u128_dec_serde};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct HexU64 {
        #[serde(with = "u64_hex_serde")]
        inner: u64,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct DecU128 {
        #[serde(with = "u128_dec_serde")]
        inner: Option<u128>,
    }

    #[test]
    fn test_u64_hex_serde() {
        let x = HexU64 { inner: 123456 };
        let x_ser = serde_json::to_string(&x).unwrap();
        let x_desr: HexU64 = serde_json::from_str(&x_ser).unwrap();
        assert_eq!(x, x_desr);
        assert_eq!(x_ser, r#"{"inner":"0x1e240"}"#);

        let missing_0x = r#"{"inner":"1e240"}"#;
        let invalid_char = r#"{"inner":"0x1q240"}"#;

        let err: Result<HexU64, _> = serde_json::from_str(missing_0x);
        assert!(err.is_err());
        assert!(format!("{:?}", err).contains("Missing 0x Prefix"));

        let err: Result<HexU64, _> = serde_json::from_str(invalid_char);
        assert!(err.is_err());
        assert!(format!("{:?}", err).contains("Invalid character 'q'"));
    }

    #[test]
    fn test_u128_dec_serde() {
        let x = DecU128 { inner: None };
        let x_ser = serde_json::to_string(&x).unwrap();
        let x_desr: DecU128 = serde_json::from_str(&x_ser).unwrap();
        assert_eq!(x, x_desr);
        assert_eq!(x_ser, r#"{"inner":null}"#);

        let x = DecU128 {
            inner: Some(123456),
        };
        let x_ser = serde_json::to_string(&x).unwrap();
        let x_desr: DecU128 = serde_json::from_str(&x_ser).unwrap();
        assert_eq!(x, x_desr);
        assert_eq!(x_ser, r#"{"inner":"123456"}"#);

        let negative_number = r#"{"inner":"-123"}"#;
        let invalid_number = r#"{"inner":"123a"}"#;

        let err: Result<DecU128, _> = serde_json::from_str(negative_number);
        assert!(err.is_err());
        assert!(format!("{:?}", err).contains("invalid digit found in string"));

        let err: Result<DecU128, _> = serde_json::from_str(invalid_number);
        assert!(err.is_err());
        assert!(format!("{:?}", err).contains("invalid digit found in string"));
    }
}
