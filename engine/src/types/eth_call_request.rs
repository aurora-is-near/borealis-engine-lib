use aurora_engine_transactions::eip_7702::{
    AuthorizationTuple, SignedTransaction7702, Transaction7702,
};
use aurora_engine_types::types::{Address, Wei};
use aurora_engine_types::{H160, H256, U256};
use aurora_evm::executor::stack::Authorization;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthCallRequest {
    pub from: Address,
    pub to: Option<Address>,
    pub gas_limit: GasLimit,
    pub gas_price: U256,
    pub value: Wei,
    pub data: Vec<u8>,
    pub block_id: BlockId,
    pub nonce: Option<u64>,
    pub state_override: Vec<(Address, StateOverride)>,
    pub access_list: Vec<(H160, Vec<H256>)>,
    pub authorization_list: Vec<AuthorizationTuple>,
}

use crate::types::{BlockId, GasLimit, StateOverride};

impl EthCallRequest {
    pub const DEFAULT_GAS_LIMIT: U256 = U256([u64::MAX, 0, 0, 0]);

    pub fn from_json_value(body_obj: &serde_json::Map<String, serde_json::Value>) -> Option<Self> {
        let params = body_obj.get("params")?.as_array()?;
        let params_obj = params.first()?.as_object()?;
        let from = Self::parse_address(params_obj, "from").flatten()?;
        let to = Self::parse_address(params_obj, "to")?;
        let gas_limit = if !params_obj.contains_key("gas") {
            GasLimit::Default(Self::DEFAULT_GAS_LIMIT.low_u64())
        } else {
            let value = Self::parse_hex_int(params_obj, "gas", None)?.low_u64();
            GasLimit::UserDefined(value)
        };
        let gas_price = Self::parse_hex_int(params_obj, "gasPrice", Some(U256::zero()))?;
        let value = Self::parse_hex_int(params_obj, "value", Some(U256::zero())).map(Wei::new)?;
        let data = Self::parse_hex_bytes(params_obj, ["data", "input"])?;
        let nonce = Self::parse_hex_int(params_obj, "nonce", None).map(|x| x.low_u64());
        let block_id = BlockId::from_json_value(params.get(1))?;
        let state_override = StateOverride::from_json_value(params.get(2))?;
        let access_list = Self::parse_list::<AccessItem, _>(params_obj, "accessList");
        let authorization_list =
            Self::parse_list::<AuthorizationItem, _>(params_obj, "authorizationList");

        Some(Self {
            from,
            to,
            gas_limit,
            gas_price,
            value,
            data,
            block_id,
            nonce,
            state_override,
            access_list,
            authorization_list,
        })
    }

    fn parse_address(
        body_obj: &serde_json::Map<String, serde_json::Value>,
        field: &str,
    ) -> Option<Option<Address>> {
        Some(match body_obj.get(field) {
            None | Some(serde_json::Value::Null) if field == "to" => None,
            None | Some(serde_json::Value::Null) => Some(Address::default()),
            Some(value) => {
                let hex_str = value
                    .as_str()
                    .map(|hex_str| hex_str.strip_prefix("0x").unwrap_or(hex_str))?;

                if hex_str.is_empty() {
                    Some(Address::default())
                } else {
                    Address::decode(hex_str).ok()
                }
            }
        })
    }

    fn parse_hex_int(
        body_obj: &serde_json::Map<String, serde_json::Value>,
        field: &str,
        default: Option<U256>,
    ) -> Option<U256> {
        match body_obj.get(field) {
            None => default,
            Some(value) => {
                let hex_str = value.as_str()?;
                let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                U256::from_str_radix(hex_str, 16).ok()
            }
        }
    }

    fn parse_hex_bytes<'a, I>(
        body_obj: &serde_json::Map<String, serde_json::Value>,
        // List of possible fields where the data might be found.
        // The first one in the list that is present will be used, others ignored.
        fields: I,
    ) -> Option<Vec<u8>>
    where
        I: IntoIterator<Item = &'a str>,
    {
        for field in fields {
            if let Some(value) = body_obj.get(field) {
                let hex_str = value.as_str()?;
                let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                return hex::decode(hex_str.to_lowercase()).ok();
            }
        }
        Some(Vec::new())
    }

    fn parse_list<T: DeserializeOwned, R: From<T>>(
        body_obj: &serde_json::Map<String, serde_json::Value>,
        field: &str,
    ) -> Vec<R> {
        body_obj
            .get(field)
            .and_then(|value| serde_json::from_value::<Vec<T>>(value.clone()).ok())
            .map_or(vec![], |list| list.into_iter().map(Into::into).collect())
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessItem {
    address: H160,
    storage_keys: Vec<H256>,
}

impl From<AccessItem> for (H160, Vec<H256>) {
    fn from(value: AccessItem) -> Self {
        (value.address, value.storage_keys)
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationItem {
    chain_id: U256,
    address: H160,
    nonce: U256,
    y_parity: U256,
    r: U256,
    s: U256,
}

impl From<AuthorizationItem> for AuthorizationTuple {
    fn from(value: AuthorizationItem) -> Self {
        Self {
            chain_id: value.chain_id,
            address: value.address,
            nonce: if value.nonce > U256::from(u64::MAX) {
                u64::MAX
            } else {
                value.nonce.as_u64()
            },
            parity: value.y_parity,
            r: value.r,
            s: value.s,
        }
    }
}

/// Convert a list of `AuthorizationTuple` to `Vec<Authorization>` which is needed for EVM execution.
pub fn convert_authorization_list(
    list: &[AuthorizationTuple],
    chain_id: [u8; 32],
) -> Vec<Authorization> {
    let chain_id_u256 = U256::from_big_endian(&chain_id);
    SignedTransaction7702 {
        transaction: Transaction7702 {
            chain_id: if chain_id_u256 > U256::from(u64::MAX) {
                u64::MAX
            } else {
                chain_id_u256.as_u64()
            },
            nonce: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            max_fee_per_gas: Default::default(),
            gas_limit: Default::default(),
            to: Default::default(),
            value: Default::default(),
            data: vec![],
            access_list: vec![],
            authorization_list: list.to_owned(),
        },
        parity: 0,
        r: Default::default(),
        s: Default::default(),
    }
    .authorization_list()
    .unwrap_or_default()
}

#[test]
fn test_deserialize_eth_call_request() {
    let value = serde_json::from_str::<serde_json::Value>(
        r#"{
  "jsonrpc": "2.0",
  "method": "eth_call",
  "params": [
    {
      "from": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
      "to": "0x6b175474e89094c44da98b954eedeac495271d0f",
      "data": "0xa9059cbb000000000000000000000000C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2000000000000000000000000000000000000000000000000de0b6b3a76400000",
      "value": "0x0",
      "accessList": [
        {
          "address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
          "storageKeys": [
            "0x0000000000000000000000000000000000000000000000000000000000000003",
            "0x0000000000000000000000000000000000000000000000000000000000000007"
          ]
        }
      ],
      "authorizationList": [
        {
          "chainId": "0x1",
          "address": "0x5FbDB2315678afecb367f032d93F642f64180aa3",
          "nonce": "0xFF",
          "yParity": "0x1",
          "r": "0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
          "s": "0x53965653d309193108c9d2c6c0987a0c102a900db84236e765507713d33454b3"
        }
      ]
    },
    "latest"
  ],
  "id": 1
}"#,
    ).unwrap();
    let map = value.as_object().unwrap();
    let eth_request = EthCallRequest::from_json_value(map).unwrap();

    assert_eq!(eth_request.access_list.len(), 1);
    assert_eq!(eth_request.authorization_list.len(), 1);
}
