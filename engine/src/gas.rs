use aurora_engine::{
    engine::{Engine, EngineError, EngineErrorKind},
    parameters::SubmitResult,
};
use aurora_engine_modexp::AuroraModExp;
use aurora_engine_sdk::io::IO;
use aurora_engine_transactions::NormalizedEthTransaction;
use aurora_engine_types::{
    storage,
    types::{Address, Wei},
    H160, H256, U256,
};
use engine_standalone_storage::{
    engine_state::{EngineStateAccess, EngineStorageValue},
    Storage,
};
use std::collections::HashMap;

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

fn parse_hex_bytes(
    body_obj: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Option<Vec<u8>> {
    match body_obj.get(field) {
        None => Some(Vec::new()),
        Some(value) => {
            let hex_str = value.as_str()?;
            let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            hex::decode(hex_str).ok()
        }
    }
}

fn parse_h256_map<T, F: Fn(H256, H256, &mut T)>(
    body_obj: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    mut result: T,
    insert_fn: F,
) -> Option<T> {
    let inner_map = body_obj.get(field)?.as_object()?;
    for (k, v) in inner_map.iter() {
        let k = parse_h256(k)?;
        let v = v.as_str().and_then(parse_h256)?;
        insert_fn(k, v, &mut result);
    }
    Some(result)
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

fn parse_address(hex_str: &str) -> Option<Address> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    Address::decode(hex_str).ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateOverride {
    balance: Option<U256>,
    nonce: Option<U256>,
    code: Option<Vec<u8>>,
    state: Option<HashMap<H256, H256>>,
    state_diff: Option<Vec<(H256, H256)>>,
}

impl StateOverride {
    pub fn from_json_value(value: Option<&serde_json::Value>) -> Option<Vec<(Address, Self)>> {
        let state_object = match value.and_then(|v| v.as_object()) {
            Some(v) => v,
            None => return Some(Vec::new()),
        };

        let mut result = Vec::with_capacity(state_object.len());
        for (k, v) in state_object.iter() {
            let address = parse_address(k)?;
            let override_object = v.as_object()?;
            let state_override = Self::parse_single_override(override_object)?;
            result.push((address, state_override));
        }

        Some(result)
    }

    fn parse_single_override(
        override_object: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<Self> {
        let balance = parse_hex_int(override_object, "balance", None);
        let nonce = parse_hex_int(override_object, "nonce", None);
        let code = parse_hex_bytes(override_object, "code");
        let state = parse_h256_map(override_object, "state", HashMap::new(), |k, v, c| {
            c.insert(k, v);
        });
        let state_diff = parse_h256_map(override_object, "stateDiff", Vec::new(), |k, v, c| {
            c.push((k, v));
        });
        Some(Self {
            balance,
            nonce,
            code,
            state,
            state_diff,
        })
    }
}

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
            // Block Id can be a string or object as per https://eips.ethereum.org/EIPS/eip-1898
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthCallRequest {
    pub from: Address,
    pub to: Option<Address>,
    pub gas_limit: u64,
    pub gas_price: U256,
    pub value: Wei,
    pub data: Vec<u8>,
    pub block_id: BlockId,
    pub nonce: Option<u64>,
    pub state_override: Vec<(Address, StateOverride)>,
}

impl EthCallRequest {
    const DEFAULT_GAS_LIMIT: U256 = U256([u64::MAX, 0, 0, 0]);

    pub fn from_json_value(body: serde_json::Value) -> Option<Self> {
        let params = body.as_object()?.get("params")?.as_array()?;
        let params_obj = params.first()?.as_object()?;
        let from = Self::parse_address(params_obj, "from")?;
        let to = if params_obj.contains_key("to") {
            Some(Self::parse_address(params_obj, "to")?)
        } else {
            None
        };
        let gas_limit = parse_hex_int(params_obj, "gas", Some(Self::DEFAULT_GAS_LIMIT))?.low_u64();
        let gas_price = parse_hex_int(params_obj, "gasPrice", Some(U256::zero()))?;
        let value = parse_hex_int(params_obj, "value", Some(U256::zero())).map(Wei::new)?;
        let data = parse_hex_bytes(params_obj, "data")?;
        let nonce = parse_hex_int(params_obj, "nonce", None).map(|x| x.low_u64());
        let block_id = BlockId::from_json_value(params.get(1))?;
        let state_override = StateOverride::from_json_value(params.get(2))?;

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
        })
    }

    fn parse_address(
        body_obj: &serde_json::Map<String, serde_json::Value>,
        field: &str,
    ) -> Option<Address> {
        let hex_str = match body_obj.get(field) {
            None | Some(serde_json::Value::Null) => "",
            Some(value) => value.as_str()?,
        };
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        if hex_str.is_empty() {
            return Some(Address::zero());
        }
        Address::decode(hex_str).ok()
    }
}

#[allow(clippy::too_many_arguments)]
fn eth_call(
    from: Address,
    to: Option<Address>,
    gas_limit: u64,
    gas_price: U256,
    value: Wei,
    data: Vec<u8>,
    storage: &Storage,
    block_id: BlockId,
    nonce: Option<u64>,
    earliest_block_height: u64,
    state_override: Vec<(Address, StateOverride)>,
) -> (Result<SubmitResult, StateOrEngineError>, NonceStatus) {
    let (block_hash, block_height) = match block_id {
        BlockId::Number(b) => (storage.get_block_hash_by_height(b).unwrap_or_default(), b),
        BlockId::Hash(h) => (
            h,
            storage
                .get_block_height_by_hash(h)
                .unwrap_or(earliest_block_height),
        ),
        BlockId::Latest => storage.get_latest_block().unwrap_or_default(),
        BlockId::Earliest => {
            let height = earliest_block_height;
            (
                storage.get_block_hash_by_height(height).unwrap_or_default(),
                height,
            )
        }
    };
    let block_metadata = storage.get_block_metadata(block_hash).unwrap_or_else(|_| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let random_seed = aurora_engine_sdk::keccak(&nanos.to_be_bytes());
        engine_standalone_storage::BlockMetadata {
            timestamp: aurora_engine_sdk::env::Timestamp::new(nanos as u64),
            random_seed,
        }
    });
    let default_account_id: aurora_engine_types::account_id::AccountId = "system".parse().unwrap();
    let current_account_id = "aurora".parse().unwrap();
    let env = aurora_engine_sdk::env::Fixed {
        signer_account_id: default_account_id.clone(),
        current_account_id,
        predecessor_account_id: default_account_id,
        block_height,
        block_timestamp: block_metadata.timestamp,
        attached_deposit: 1,
        random_seed: block_metadata.random_seed,
        prepaid_gas: aurora_engine_types::types::NearGas::new(300),
    };
    storage
        .with_engine_access(block_height + 1, 0, &[], |io| {
            let current_nonce = aurora_engine::engine::get_nonce(&io, &from).low_u64();
            let mut local_io = io;
            let mut full_override = HashMap::new();
            for (address, state_override) in state_override {
                if let Some(balance) = state_override.balance {
                    aurora_engine::engine::set_balance(&mut local_io, &address, &Wei::new(balance));
                }
                if let Some(nonce) = state_override.nonce {
                    aurora_engine::engine::set_nonce(&mut local_io, &address, &nonce);
                }
                if let Some(code) = state_override.code {
                    aurora_engine::engine::set_code(&mut local_io, &address, &code);
                }
                if let Some(state) = state_override.state {
                    full_override.insert(address.raw(), state);
                }
                if let Some(state_diff) = state_override.state_diff {
                    let generation = aurora_engine::engine::get_generation(&local_io, &address);
                    for (k, v) in state_diff {
                        aurora_engine::engine::set_storage(
                            &mut local_io,
                            &address,
                            &k,
                            &v,
                            generation,
                        );
                    }
                }
            }

            let submit_result = if full_override.is_empty() {
                compute_call_result(
                    local_io, from, to, gas_limit, gas_price, value, data, nonce, env,
                )
            } else {
                let override_io = EngineStateOverride {
                    inner: local_io,
                    state_override: &full_override,
                };
                compute_call_result(
                    override_io,
                    from,
                    to,
                    gas_limit,
                    gas_price,
                    value,
                    data,
                    nonce,
                    env,
                )
            };
            let nonce_status = nonce.map_or(NonceStatus::NotProvided { current_nonce }, |nonce| {
                if nonce < current_nonce {
                    NonceStatus::TooLow
                } else {
                    NonceStatus::GreaterOrEqual { current_nonce }
                }
            });
            (submit_result, nonce_status)
        })
        .result
}

#[derive(Clone, Copy)]
pub struct EngineStateOverride<'db, 'input, 'output, 'state> {
    pub inner: EngineStateAccess<'db, 'input, 'output>,
    pub state_override: &'state HashMap<H160, HashMap<H256, H256>>,
}

impl<'db, 'input: 'db, 'output: 'db, 'state> IO
    for EngineStateOverride<'db, 'input, 'output, 'state>
{
    type StorageValue = EngineStorageValue<'db>;

    fn read_input(&self) -> Self::StorageValue {
        self.inner.read_input()
    }

    fn return_output(&mut self, value: &[u8]) {
        self.inner.return_output(value)
    }

    fn read_storage(&self, key: &[u8]) -> Option<Self::StorageValue> {
        match deconstruct_storage_key(key) {
            None => self.inner.read_storage(key),
            Some((address, index)) => self.state_override.get(&address).map_or_else(
                || self.inner.read_storage(key),
                |state_override| {
                    state_override
                        .get(&index)
                        .map(|value| EngineStorageValue::Vec(value.as_bytes().to_vec()))
                },
            ),
        }
    }

    fn storage_has_key(&self, key: &[u8]) -> bool {
        self.read_storage(key).is_some()
    }

    fn write_storage(&mut self, key: &[u8], value: &[u8]) -> Option<Self::StorageValue> {
        self.inner.write_storage(key, value)
    }

    fn write_storage_direct(
        &mut self,
        key: &[u8],
        value: Self::StorageValue,
    ) -> Option<Self::StorageValue> {
        self.inner.write_storage_direct(key, value)
    }

    fn remove_storage(&mut self, key: &[u8]) -> Option<Self::StorageValue> {
        self.inner.remove_storage(key)
    }
}

const STORAGE_VERSION: u8 = storage::VersionPrefix::V1 as u8;
const STORAGE_PREFIX: u8 = storage::KeyPrefix::Storage as u8;

fn deconstruct_storage_key(key: &[u8]) -> Option<(H160, H256)> {
    let version = *key.first()?;
    if version != STORAGE_VERSION {
        panic!("Unexpected version");
    }
    if key.get(1)? == &STORAGE_PREFIX {
        let key_len = key.len();
        // Lengths are 54 or 58 bytes, depending on if the generation is present or not
        if key_len == 54 {
            let address = H160::from_slice(&key[2..22]);
            let value = H256::from_slice(&key[22..54]);
            Some((address, value))
        } else if key_len == 58 {
            let address = H160::from_slice(&key[2..22]);
            let value = H256::from_slice(&key[26..58]);
            Some((address, value))
        } else {
            panic!("Unexpected storage key length")
        }
    } else {
        None
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "request_nonce_type")]
pub enum NonceStatus {
    NotProvided { current_nonce: u64 },
    TooLow,
    GreaterOrEqual { current_nonce: u64 },
}

#[derive(Debug, serde::Serialize)]
pub enum StateOrEngineError {
    StateMissing,
    Engine(EngineError),
}

#[allow(clippy::too_many_arguments)]
fn compute_call_result<I: aurora_engine_sdk::io::IO + Copy>(
    io: I,
    from: Address,
    to: Option<Address>,
    gas_limit: u64,
    gas_price: U256,
    value: Wei,
    data: Vec<u8>,
    nonce: Option<u64>,
    env: aurora_engine_sdk::env::Fixed,
) -> Result<SubmitResult, StateOrEngineError> {
    let mut handler = aurora_engine_sdk::promise::Noop;
    aurora_engine::state::get_state(&io)
        .map_err(|_| StateOrEngineError::StateMissing)
        .and_then(|engine_state| {
            let mut engine: Engine<_, _, AuroraModExp> =
                aurora_engine::engine::Engine::new_with_state(
                    engine_state,
                    from,
                    env.current_account_id.clone(),
                    io,
                    &env,
                );
            let result = match to {
                Some(to) => engine
                    .call(&from, &to, value, data, gas_limit, Vec::new(), &mut handler)
                    .map_err(StateOrEngineError::Engine),
                None => engine
                    .deploy_code(from, value, data, None, gas_limit, Vec::new(), &mut handler)
                    .map_err(StateOrEngineError::Engine),
            };
            if !gas_price.is_zero() && result.is_ok() {
                let gas_used = result.as_ref().unwrap().gas_used;
                let gas_estimate = gas_used.saturating_add(gas_used / 3);
                let transaction = NormalizedEthTransaction {
                    address: from,
                    chain_id: None,
                    nonce: nonce.map(U256::from).unwrap_or_default(),
                    gas_limit: U256::from(gas_estimate),
                    max_priority_fee_per_gas: gas_price,
                    max_fee_per_gas: gas_price,
                    to,
                    value,
                    // We do not use the real `data` here to avoid moving it before passing to `call`.
                    // It is ok to not have the `data` here because it is not used by the `charge_gas` function.
                    data: Vec::new(),
                    access_list: Vec::new(),
                };
                engine
                    .charge_gas(&from, &transaction, None, None)
                    .map_err(|e| {
                        StateOrEngineError::Engine(EngineErrorKind::GasPayment(e).into())
                    })?;
            }
            result
        })
}

pub fn estimate_gas(
    storage: &Storage,
    request: EthCallRequest,
    earliest_block_height: u64,
) -> (Result<SubmitResult, StateOrEngineError>, NonceStatus) {
    let (result, nonce) = eth_call(
        request.from,
        request.to,
        u64::MAX,
        U256::zero(),
        request.value,
        request.data.clone(),
        storage,
        request.block_id,
        request.nonce,
        earliest_block_height,
        request.state_override.clone(),
    );

    // If the request gas price was 0 then there is no reason to try again.
    // The only reason to retry is to see if the user has enough ETH balance to cover
    // the gas cost with the estimated limit.
    if request.gas_price.is_zero() {
        return (result, nonce);
    }

    match result {
        Ok(submit_result) => {
            let computed_gas_limit = submit_result
                .gas_used
                .saturating_add(submit_result.gas_used / 3);
            eth_call(
                request.from,
                request.to,
                computed_gas_limit,
                request.gas_price,
                request.value,
                request.data,
                storage,
                request.block_id,
                request.nonce,
                earliest_block_height,
                request.state_override,
            )
        }

        Err(_) => (result, nonce),
    }
}
