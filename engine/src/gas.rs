use crate::types::{BlockId, EthCallRequest, convert_authorization_list};
use aurora_engine::{
    engine::{Engine, EngineError, EngineErrorKind},
    parameters::SubmitResult,
};
use aurora_engine_modexp::AuroraModExp;
use aurora_engine_sdk::io::IO;
use aurora_engine_transactions::NormalizedEthTransaction;
use aurora_engine_types::{
    H160, H256, U256, storage,
    types::{NearGas, Wei},
};
use engine_standalone_storage::{
    Storage,
    engine_state::{EngineStateAccess, EngineStorageValue},
};
use std::collections::HashMap;

/// Function for estimation gas.
pub fn estimate_gas(
    storage: &Storage,
    mut request: EthCallRequest,
    earliest_block_height: u64,
) -> (Result<SubmitResult, StateOrEngineError>, NonceStatus) {
    let actual_gas_price = request.gas_price;
    request.gas_price = U256::zero();

    let (result, nonce) = eth_call(storage, request.clone(), u64::MAX, earliest_block_height);

    // If the request gas_price is 0, then there is no reason to try again.
    // The only reason to retry is to see if the user has enough ETH to cover
    // the gas cost with the estimated limit.
    match result {
        Ok(res) if !actual_gas_price.is_zero() => {
            let gas_used = res.gas_used;
            let computed_gas_limit = gas_used.saturating_add(gas_used / 3);
            request.gas_price = actual_gas_price;

            eth_call(storage, request, computed_gas_limit, earliest_block_height)
        }
        _ => (result, nonce),
    }
}

fn eth_call(
    storage: &Storage,
    request: EthCallRequest,
    gas_limit: u64,
    earliest_block_height: u64,
) -> (Result<SubmitResult, StateOrEngineError>, NonceStatus) {
    let (block_hash, block_height) = match request.block_id {
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
    let current_account_id = storage.get_engine_account_id().unwrap();
    let env = aurora_engine_sdk::env::Fixed {
        signer_account_id: default_account_id.clone(),
        current_account_id,
        predecessor_account_id: default_account_id,
        block_height,
        block_timestamp: block_metadata.timestamp,
        attached_deposit: 1,
        random_seed: block_metadata.random_seed,
        prepaid_gas: NearGas::new(300),
        used_gas: NearGas::new(0),
    };
    storage
        .with_engine_access(block_height + 1, 0, &[], |io| {
            let current_nonce = aurora_engine::engine::get_nonce(&io, &request.from).low_u64();
            let mut local_io = io;
            let mut full_override = HashMap::new();
            for (address, state_override) in &request.state_override {
                if let Some(balance) = state_override.balance {
                    aurora_engine::engine::set_balance(&mut local_io, address, &Wei::new(balance));
                }
                if let Some(nonce) = state_override.nonce {
                    aurora_engine::engine::set_nonce(&mut local_io, address, &nonce);
                }
                if let Some(code) = &state_override.code {
                    aurora_engine::engine::set_code(&mut local_io, address, code);
                }
                if let Some(state) = &state_override.state {
                    full_override.insert(address.raw(), state.clone());
                }
                if let Some(state_diff) = &state_override.state_diff {
                    let generation = aurora_engine::engine::get_generation(&local_io, address);
                    for (k, v) in state_diff {
                        aurora_engine::engine::set_storage(
                            &mut local_io,
                            address,
                            k,
                            v,
                            generation,
                        );
                    }
                }
            }
            let nonce = request.nonce;
            let submit_result = if full_override.is_empty() {
                compute_call_result(local_io, env, request, gas_limit)
            } else {
                let override_io = EngineStateOverride {
                    inner: local_io,
                    state_override: &full_override,
                };
                compute_call_result(override_io, env, request, gas_limit)
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

impl<'db, 'input: 'db, 'output: 'db> IO for EngineStateOverride<'db, 'input, 'output, '_> {
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

fn compute_call_result<I: IO + Copy>(
    io: I,
    env: aurora_engine_sdk::env::Fixed,
    request: EthCallRequest,
    gas_limit: u64,
) -> Result<SubmitResult, StateOrEngineError> {
    let mut handler = aurora_engine_sdk::promise::Noop;
    aurora_engine::state::get_state(&io)
        .map_err(|_| StateOrEngineError::StateMissing)
        .and_then(|engine_state| {
            let chain_id = engine_state.chain_id;
            let mut engine: Engine<_, _, AuroraModExp> = Engine::new_with_state(
                engine_state,
                request.from,
                env.current_account_id.clone(),
                io,
                &env,
            );
            let result = match request.to {
                Some(to) => engine
                    .call(
                        &request.from,
                        &to,
                        request.value,
                        request.data,
                        gas_limit,
                        request.access_list.clone(),
                        convert_authorization_list(&request.authorization_list, chain_id),
                        &mut handler,
                    )
                    .map_err(StateOrEngineError::Engine),
                None => engine
                    .deploy_code(
                        request.from,
                        request.value,
                        request.data,
                        None,
                        gas_limit,
                        request.access_list.clone(),
                        &mut handler,
                    )
                    .map_err(StateOrEngineError::Engine),
            };
            if !request.gas_price.is_zero() && result.is_ok() {
                let gas_used = result.as_ref().unwrap().gas_used;
                let gas_estimate = gas_used.saturating_add(gas_used / 3);
                let transaction = NormalizedEthTransaction {
                    address: request.from,
                    chain_id: None,
                    nonce: request.nonce.map(U256::from).unwrap_or_default(),
                    gas_limit: U256::from(gas_estimate),
                    max_priority_fee_per_gas: request.gas_price,
                    max_fee_per_gas: request.gas_price,
                    to: request.to,
                    value: request.value,
                    // We do not use the real `data` here to avoid moving it before passing to `call`.
                    // It is ok to not have the `data`, `access_list` and `authorization_list` here
                    // because it is not used by the `charge_gas` function.
                    data: Vec::new(),
                    access_list: Vec::new(),
                    authorization_list: Vec::new(),
                };
                engine
                    .charge_gas(&request.from, &transaction, None, None)
                    .map_err(|e| {
                        StateOrEngineError::Engine(EngineErrorKind::GasPayment(e).into())
                    })?;
            }
            result
        })
}
