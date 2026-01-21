use aurora_engine_types::{H256, U256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockId {
    Number(u64),
    Hash(H256),
    Latest,
    Earliest,
}

impl BlockId {
    pub fn from_json_value(value: Option<&serde_json::Value>) -> Option<Self> {
        match value {
            None => Some(Self::Latest),
            // BlockId can be a string or object as per https://eips.ethereum.org/EIPS/eip-1898
            Some(serde_json::Value::String(value)) => {
                let value = value.to_lowercase();
                match value.as_str() {
                    "latest" => Some(Self::Latest),
                    "earliest" => Some(Self::Earliest),
                    "pending" => Some(Self::Latest),
                    hex_str if hex_str.starts_with("0x") => {
                        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                        let block_height = U256::from_str_radix(hex_str, 16).ok()?;
                        Some(Self::Number(block_height.low_u64()))
                    }
                    _ => None,
                }
            }
            Some(serde_json::Value::Object(value)) => {
                if let Some(serde_json::Value::String(value)) = value.get("blockNumber") {
                    let value = value.to_lowercase();
                    let maybe_hex_str = value.as_str();
                    if maybe_hex_str.starts_with("0x") {
                        let hex_str = maybe_hex_str.strip_prefix("0x").unwrap_or(maybe_hex_str);
                        let block_height = U256::from_str_radix(hex_str, 16).ok()?;
                        return Some(Self::Number(block_height.low_u64()));
                    }
                }

                if let Some(serde_json::Value::String(value)) = value.get("blockHash") {
                    let value = value.to_lowercase();
                    let maybe_hex_str = value.as_str();
                    if maybe_hex_str.starts_with("0x") {
                        let hex_str = maybe_hex_str.strip_prefix("0x").unwrap_or(maybe_hex_str);
                        let bytes = hex::decode(hex_str).ok()?;
                        if bytes.len() != 32 {
                            return None;
                        }
                        return Some(Self::Hash(H256::from_slice(&bytes)));
                    }
                }

                None
            }
            // Also allow a regular number
            Some(serde_json::Value::Number(n)) => n.as_u64().map(Self::Number),
            Some(_) => None,
        }
    }
}
