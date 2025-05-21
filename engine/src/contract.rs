use std::sync::Arc;

use aurora_engine::contract_methods::ContractError;
use engine_standalone_storage::native_ffi;
use near_primitives::types::AccountId;
use near_vm_runner::{
    ContractCode,
    logic::{ReturnData, errors::VMRunnerError, mocks::mock_external::MockedExternal},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("{0}")]
    Load(#[from] native_ffi::LibLoadingError),
    #[error("{0}")]
    VM(#[from] VMRunnerError),
    #[error("wasm VM return data is bad")]
    BadReturnData,
    #[error("bad native library: {0:?}")]
    BadNativeLibrary(ContractError),
}

#[cfg(target_os = "macos")]
const LIB_SUFFIX: &str = "dylib";
#[cfg(target_os = "linux")]
const LIB_SUFFIX: &str = "so";

pub fn load() -> Result<(), UpdateError> {
    let version = "3.9.0";
    let path = format!("libaurora_engine_native_{version}.{LIB_SUFFIX}");

    native_ffi::load(path)?;

    Ok(())
}

pub fn update(wasm_code: Vec<u8>) -> Result<(), UpdateError> {
    let runtime_config_store = near_parameters::RuntimeConfigStore::test();
    let runtime_config =
        runtime_config_store.get_config(near_primitives_core::version::PROTOCOL_VERSION);
    let wasm_config = runtime_config.wasm_config.clone();
    let caller_id: AccountId = "caller.near".parse().unwrap();
    let context = near_vm_runner::logic::VMContext {
        current_account_id: caller_id.clone(),
        signer_account_id: caller_id.clone(),
        signer_account_pk: vec![],
        predecessor_account_id: caller_id,
        input: vec![],
        promise_results: Arc::new([]),
        block_height: 0,
        block_timestamp: 0,
        epoch_height: 0,
        account_balance: 10u128.pow(25),
        account_locked_balance: 0,
        storage_usage: 100,
        attached_deposit: 0,
        prepaid_gas: 10u64.pow(18),
        random_seed: vec![],
        output_data_receivers: vec![],
        view_config: None,
    };
    let mut underlying = MockedExternal::with_code(ContractCode::new(wasm_code, None));
    let contract = near_vm_runner::prepare(
        &underlying,
        wasm_config.clone(),
        None,
        context.make_gas_counter(wasm_config.as_ref()),
        "get_version",
    );

    let outcome = near_vm_runner::run(
        contract,
        &mut underlying,
        &context,
        std::sync::Arc::new(near_parameters::RuntimeFeesConfig::test()),
    )?;

    if let ReturnData::Value(version) = outcome.return_data {
        let version = std::str::from_utf8(&version)
            .map_err(|_| UpdateError::BadReturnData)?
            .trim_end();
        let path = format!("libaurora_engine_native_{version}.{LIB_SUFFIX}");
        native_ffi::load(path)?;

        native_ffi::lock()
            .get_version()
            .map_err(UpdateError::BadNativeLibrary)?;
        let native_version = native_ffi::state().take_output();
        let native_version =
            String::from_utf8(native_version).map_err(|_| UpdateError::BadReturnData)?;
        let native_version = native_version.trim_end();
        tracing::info!(
            "Update contract library: expected wasm version: {version}, actual native version: {native_version}"
        );

        Ok(())
    } else {
        Err(UpdateError::BadReturnData)
    }
}
