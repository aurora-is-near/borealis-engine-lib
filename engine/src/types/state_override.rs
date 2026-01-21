use aurora_engine_types::types::Address;
use aurora_engine_types::{H256, U256};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateOverride {
    pub balance: Option<U256>,
    pub nonce: Option<U256>,
    pub code: Option<Vec<u8>>,
    pub state: Option<HashMap<H256, H256>>,
    pub state_diff: Option<Vec<(H256, H256)>>,
}

impl StateOverride {
    pub fn from_json_value(value: Option<&serde_json::Value>) -> Option<Vec<(Address, Self)>> {
        let state_object = match value.and_then(|v| v.as_object()) {
            Some(v) => v,
            None => {
                return Some(Vec::new());
            }
        };

        let mut result = Vec::with_capacity(state_object.len());
        for (k, v) in state_object.iter() {
            let address = Self::parse_address(k)?;
            let override_object = v.as_object()?;
            let state_override = Self::parse_single_override(override_object)?;
            result.push((address, state_override));
        }

        Some(result)
    }

    fn parse_single_override(
        override_object: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<Self> {
        let balance = Self::parse_hex_int(override_object, "balance")
            .transpose()
            .ok()?;
        let nonce = Self::parse_hex_int(override_object, "nonce")
            .transpose()
            .ok()?;
        let code = Self::parse_hex_bytes(override_object, "code")
            .transpose()
            .ok()?;
        let state = Self::parse_h256_map(override_object, "state", HashMap::new(), |k, v, c| {
            c.insert(k, v);
        })
        .transpose()
        .ok()?;
        let state_diff =
            Self::parse_h256_map(override_object, "stateDiff", Vec::new(), |k, v, c| {
                c.push((k, v));
            })
            .transpose()
            .ok()?;
        Some(Self {
            balance,
            nonce,
            code,
            state,
            state_diff,
        })
    }

    fn parse_address(hex_str: &str) -> Option<Address> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        Address::decode(hex_str).ok()
    }

    fn parse_h256(hex_str: &str) -> Option<H256> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        if hex_str.len() != 64 {
            return None;
        }
        hex::decode(hex_str)
            .map(|bytes| H256::from_slice(&bytes))
            .ok()
    }

    fn parse_hex_int(
        body_obj: &serde_json::Map<String, serde_json::Value>,
        field: &str,
    ) -> Option<Result<U256, ()>> {
        body_obj.get(field).map(|value| {
            let hex_str = value.as_str().ok_or(())?;
            let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            U256::from_str_radix(hex_str, 16).map_err(|_| ())
        })
    }

    fn parse_hex_bytes(
        body_obj: &serde_json::Map<String, serde_json::Value>,
        field: &str,
    ) -> Option<Result<Vec<u8>, ()>> {
        body_obj.get(field).map(|value| {
            let hex_str = value.as_str().ok_or(())?;
            let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            hex::decode(hex_str).map_err(|_| ())
        })
    }

    fn parse_h256_map<T, F: Fn(H256, H256, &mut T)>(
        body_obj: &serde_json::Map<String, serde_json::Value>,
        field: &str,
        mut result: T,
        insert_fn: F,
    ) -> Option<Result<T, ()>> {
        body_obj.get(field).map(|value| {
            let inner_map = value.as_object().ok_or(())?;
            for (k, v) in inner_map.iter() {
                let k = Self::parse_h256(k).ok_or(())?;
                let v = v.as_str().and_then(Self::parse_h256).ok_or(())?;
                insert_fn(k, v, &mut result);
            }
            Ok(result)
        })
    }
}
