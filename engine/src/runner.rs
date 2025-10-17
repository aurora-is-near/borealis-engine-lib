use std::{
    str,
    sync::{Arc, Mutex},
};

use aurora_engine_sdk::{
    env::{Env, Fixed, Timestamp},
    io::IO,
};
use aurora_engine_types::{H256, types::NearGas};
use engine_standalone_storage::AbstractContractRunner;
use lru::LruCache;
use near_crypto::PublicKey;
use near_jsonrpc_client::{
    errors::{JsonRpcError, JsonRpcServerError},
    methods::query::RpcQueryError,
};
use near_parameters::{RuntimeConfig, RuntimeConfigStore};
use near_primitives_core::{
    hash::CryptoHash,
    types::{AccountId, Balance, Gas, GasWeight},
};
use near_vm_runner::{
    CompiledContractInfo, Contract, ContractCode, ContractRuntimeCache,
    logic::{
        External, StorageAccessTracker, VMContext, VMLogicError, VMOutcome, ValuePtr,
        errors::VMRunnerError,
        mocks::mock_external::{MockAction, MockedValuePtr},
        types::{GlobalContractDeployMode, GlobalContractIdentifier, PromiseResult, ReceiptIndex},
    },
};
use thiserror::Error;

struct Nop;

impl External for Nop {
    fn storage_set(
        &mut self,
        _: &mut dyn StorageAccessTracker,
        _: &[u8],
        _: &[u8],
    ) -> Result<Option<Vec<u8>>, VMLogicError> {
        Ok(None)
    }

    fn storage_get<'a>(
        &'a self,
        _: &mut dyn StorageAccessTracker,
        _: &[u8],
    ) -> Result<Option<Box<dyn ValuePtr + 'a>>, VMLogicError> {
        Ok(None)
    }

    fn storage_remove(
        &mut self,
        _: &mut dyn StorageAccessTracker,
        _: &[u8],
    ) -> Result<Option<Vec<u8>>, VMLogicError> {
        Ok(None)
    }

    fn storage_has_key(
        &mut self,
        _: &mut dyn StorageAccessTracker,
        _: &[u8],
    ) -> Result<bool, VMLogicError> {
        Ok(false)
    }

    fn generate_data_id(&mut self) -> CryptoHash {
        CryptoHash::default()
    }

    fn get_recorded_storage_size(&self) -> usize {
        0
    }

    fn validator_stake(&self, _: &AccountId) -> Result<Option<Balance>, VMLogicError> {
        Ok(None)
    }

    fn validator_total_stake(&self) -> Result<Balance, VMLogicError> {
        Ok(0)
    }

    fn create_action_receipt(
        &mut self,
        _: Vec<ReceiptIndex>,
        _: AccountId,
    ) -> Result<ReceiptIndex, VMLogicError> {
        Ok(0)
    }

    fn create_promise_yield_receipt(
        &mut self,
        _: AccountId,
    ) -> Result<(ReceiptIndex, CryptoHash), VMLogicError> {
        Ok((0, CryptoHash::default()))
    }

    fn submit_promise_resume_data(
        &mut self,
        _: CryptoHash,
        _: Vec<u8>,
    ) -> Result<bool, VMLogicError> {
        Ok(false)
    }

    fn append_action_create_account(&mut self, _: ReceiptIndex) -> Result<(), VMLogicError> {
        Ok(())
    }

    fn append_action_deploy_contract(
        &mut self,
        _: ReceiptIndex,
        _: Vec<u8>,
    ) -> Result<(), VMLogicError> {
        Ok(())
    }

    fn append_action_deploy_global_contract(
        &mut self,
        _: ReceiptIndex,
        _: Vec<u8>,
        _: GlobalContractDeployMode,
    ) -> Result<(), VMLogicError> {
        Ok(())
    }

    fn append_action_use_global_contract(
        &mut self,
        _: ReceiptIndex,
        _: GlobalContractIdentifier,
    ) -> Result<(), VMLogicError> {
        Ok(())
    }

    fn append_action_function_call_weight(
        &mut self,
        _: ReceiptIndex,
        _: Vec<u8>,
        _: Vec<u8>,
        _: Balance,
        _: Gas,
        _: GasWeight,
    ) -> Result<(), VMLogicError> {
        Ok(())
    }

    fn append_action_transfer(&mut self, _: ReceiptIndex, _: Balance) -> Result<(), VMLogicError> {
        Ok(())
    }

    fn append_action_stake(&mut self, _: ReceiptIndex, _: Balance, _: PublicKey) {}

    fn append_action_add_key_with_full_access(&mut self, _: ReceiptIndex, _: PublicKey, _: u64) {}

    fn append_action_add_key_with_function_call(
        &mut self,
        _: ReceiptIndex,
        _: PublicKey,
        _: u64,
        _: Option<Balance>,
        _: AccountId,
        _: Vec<Vec<u8>>,
    ) -> Result<(), VMLogicError> {
        Ok(())
    }

    fn append_action_delete_key(&mut self, _: ReceiptIndex, _: PublicKey) {}

    fn append_action_delete_account(
        &mut self,
        _: ReceiptIndex,
        _: AccountId,
    ) -> Result<(), VMLogicError> {
        Ok(())
    }

    fn get_receipt_receiver(&self, _: ReceiptIndex) -> &AccountId {
        panic!("not implemented")
    }
}

struct EngineStateVMAccess<I: IO> {
    io: I,
    action_log: Vec<MockAction>,
}

impl<I: IO> External for EngineStateVMAccess<I>
where
    I::StorageValue: AsRef<[u8]>,
{
    fn storage_set(
        &mut self,
        _access_tracker: &mut dyn StorageAccessTracker,
        key: &[u8],
        value: &[u8],
    ) -> Result<Option<Vec<u8>>, VMLogicError> {
        Ok(self
            .io
            .write_storage(key, value)
            .map(|v| v.as_ref().to_vec()))
    }

    fn storage_get<'a>(
        &'a self,
        _access_tracker: &mut dyn StorageAccessTracker,
        key: &[u8],
    ) -> Result<Option<Box<dyn ValuePtr + 'a>>, VMLogicError> {
        Ok(self
            .io
            .read_storage(key)
            .map::<Box<dyn ValuePtr>, _>(|value| Box::new(MockedValuePtr::new(value))))
    }

    fn storage_remove(
        &mut self,
        _access_tracker: &mut dyn StorageAccessTracker,
        key: &[u8],
    ) -> Result<Option<Vec<u8>>, VMLogicError> {
        Ok(self.io.remove_storage(key).map(|v| v.as_ref().to_vec()))
    }

    fn storage_has_key(
        &mut self,
        _access_tracker: &mut dyn StorageAccessTracker,
        key: &[u8],
    ) -> Result<bool, VMLogicError> {
        Ok(self.io.storage_has_key(key))
    }

    fn generate_data_id(&mut self) -> CryptoHash {
        unimplemented!()
    }

    fn get_recorded_storage_size(&self) -> usize {
        0
    }

    fn validator_stake(&self, account_id: &AccountId) -> Result<Option<Balance>, VMLogicError> {
        let _ = account_id;
        unimplemented!()
    }

    fn validator_total_stake(&self) -> Result<Balance, VMLogicError> {
        unimplemented!()
    }

    fn create_action_receipt(
        &mut self,
        receipt_indices: Vec<ReceiptIndex>,
        receiver_id: AccountId,
    ) -> Result<ReceiptIndex, VMLogicError> {
        let index = self
            .action_log
            .len()
            .try_into()
            .expect("pointer size must fit in 64 bit");
        self.action_log.push(MockAction::CreateReceipt {
            receipt_indices,
            receiver_id,
        });
        Ok(index)
    }

    fn create_promise_yield_receipt(
        &mut self,
        receiver_id: AccountId,
    ) -> Result<(ReceiptIndex, CryptoHash), VMLogicError> {
        let index = self
            .action_log
            .len()
            .try_into()
            .expect("pointer size must fit in 64 bit");
        let data_id = self.generate_data_id();
        self.action_log.push(MockAction::YieldCreate {
            data_id,
            receiver_id,
        });
        Ok((index, data_id))
    }

    fn submit_promise_resume_data(
        &mut self,
        data_id: CryptoHash,
        data: Vec<u8>,
    ) -> Result<bool, VMLogicError> {
        self.action_log
            .push(MockAction::YieldResume { data_id, data });
        for action in &self.action_log {
            let MockAction::YieldCreate { data_id: did, .. } = action else {
                continue;
            };
            // FIXME: should also check that receiver_id matches current account_id, but there
            // isn't one tracked by `Self`...
            if data_id == *did {
                // NB: does not actually handle timeouts.
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn append_action_create_account(
        &mut self,
        receipt_index: ReceiptIndex,
    ) -> Result<(), VMLogicError> {
        self.action_log
            .push(MockAction::CreateAccount { receipt_index });
        Ok(())
    }

    fn append_action_deploy_contract(
        &mut self,
        receipt_index: ReceiptIndex,
        code: Vec<u8>,
    ) -> Result<(), VMLogicError> {
        self.action_log.push(MockAction::DeployContract {
            receipt_index,
            code,
        });
        Ok(())
    }

    fn append_action_deploy_global_contract(
        &mut self,
        receipt_index: ReceiptIndex,
        code: Vec<u8>,
        mode: GlobalContractDeployMode,
    ) -> Result<(), VMLogicError> {
        let _ = (receipt_index, code, mode);
        Ok(())
    }

    fn append_action_use_global_contract(
        &mut self,
        receipt_index: ReceiptIndex,
        contract_id: GlobalContractIdentifier,
    ) -> Result<(), VMLogicError> {
        let _ = (receipt_index, contract_id);
        Ok(())
    }

    fn append_action_function_call_weight(
        &mut self,
        receipt_index: ReceiptIndex,
        method_name: Vec<u8>,
        args: Vec<u8>,
        attached_deposit: Balance,
        prepaid_gas: Gas,
        gas_weight: GasWeight,
    ) -> Result<(), VMLogicError> {
        self.action_log.push(MockAction::FunctionCallWeight {
            receipt_index,
            method_name,
            args,
            attached_deposit,
            prepaid_gas,
            gas_weight,
        });
        Ok(())
    }

    fn append_action_transfer(
        &mut self,
        receipt_index: ReceiptIndex,
        deposit: Balance,
    ) -> Result<(), VMLogicError> {
        self.action_log.push(MockAction::Transfer {
            receipt_index,
            deposit,
        });
        Ok(())
    }

    fn append_action_stake(
        &mut self,
        receipt_index: ReceiptIndex,
        stake: Balance,
        public_key: PublicKey,
    ) {
        self.action_log.push(MockAction::Stake {
            receipt_index,
            stake,
            public_key,
        });
    }

    fn append_action_add_key_with_full_access(
        &mut self,
        receipt_index: ReceiptIndex,
        public_key: PublicKey,
        nonce: u64,
    ) {
        self.action_log.push(MockAction::AddKeyWithFullAccess {
            receipt_index,
            public_key,
            nonce,
        });
    }

    fn append_action_add_key_with_function_call(
        &mut self,
        receipt_index: ReceiptIndex,
        public_key: PublicKey,
        nonce: u64,
        allowance: Option<Balance>,
        receiver_id: AccountId,
        method_names: Vec<Vec<u8>>,
    ) -> Result<(), VMLogicError> {
        self.action_log.push(MockAction::AddKeyWithFunctionCall {
            receipt_index,
            public_key,
            nonce,
            allowance,
            receiver_id,
            method_names,
        });
        Ok(())
    }

    fn append_action_delete_key(&mut self, receipt_index: ReceiptIndex, public_key: PublicKey) {
        self.action_log.push(MockAction::DeleteKey {
            receipt_index,
            public_key,
        });
    }

    fn append_action_delete_account(
        &mut self,
        receipt_index: ReceiptIndex,
        beneficiary_id: AccountId,
    ) -> Result<(), VMLogicError> {
        self.action_log.push(MockAction::DeleteAccount {
            receipt_index,
            beneficiary_id,
        });
        Ok(())
    }

    fn get_receipt_receiver(&self, receipt_index: ReceiptIndex) -> &AccountId {
        let index: usize = receipt_index
            .try_into()
            .expect("pointer size is long enough");
        match self.action_log.get(index) {
            Some(MockAction::CreateReceipt { receiver_id, .. }) => receiver_id,
            _ => panic!("not a valid receipt index!"),
        }
    }
}

pub struct ContractRunner {
    contract: Vec<CodeWrapper>,
    runtime_config: Arc<RuntimeConfig>,
    cache: SimpleContractRuntimeCache,
}

struct CodeWrapper(Arc<ContractCode>);

impl Contract for CodeWrapper {
    fn get_code(&self) -> Option<Arc<ContractCode>> {
        Some(self.0.clone())
    }

    fn hash(&self) -> near_primitives_core::hash::CryptoHash {
        *self.0.hash()
    }
}

#[derive(Error, Debug)]
pub enum GetVersionError<E> {
    #[error("Failed to query version: {0}")]
    Inner(E),
    #[error("Received unexpected response")]
    UnexpectedResponse,
    #[error("Failed to decode UTF-8 string")]
    Utf8Error(#[from] str::Utf8Error),
    #[error("Operation timed out: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),
}

impl GetVersionError<JsonRpcError<RpcQueryError>> {
    pub const fn out_of_range(&self) -> bool {
        matches!(
            self,
            Self::Inner(JsonRpcError::ServerError(JsonRpcServerError::HandlerError(
                RpcQueryError::UnknownBlock { .. },
            )))
        )
    }
}

impl ContractRunner {
    fn new_mainnet(code: Vec<u8>, hash: Option<CryptoHash>) -> Self {
        Self::new(near_primitives_core::chains::MAINNET, code, hash)
    }

    fn new_empty() -> Self {
        let runtime_config_store =
            RuntimeConfigStore::for_chain_id(near_primitives_core::chains::MAINNET);
        let runtime_config =
            runtime_config_store.get_config(near_primitives_core::version::PROTOCOL_VERSION);
        Self {
            contract: vec![],
            runtime_config: runtime_config.clone(),
            cache: SimpleContractRuntimeCache {
                inner: Arc::new(Mutex::new(LruCache::new(
                    10.try_into().expect("`10` is non zero"),
                ))),
            },
        }
    }

    fn new(chain_id: &str, code: Vec<u8>, hash: Option<CryptoHash>) -> Self {
        let runtime_config_store = RuntimeConfigStore::for_chain_id(chain_id);
        let runtime_config =
            runtime_config_store.get_config(near_primitives_core::version::PROTOCOL_VERSION);
        Self {
            contract: vec![CodeWrapper(Arc::new(ContractCode::new(code, hash)))],
            runtime_config: runtime_config.clone(),
            cache: SimpleContractRuntimeCache {
                inner: Arc::new(Mutex::new(LruCache::new(
                    10.try_into().expect("`10` is non zero"),
                ))),
            },
        }
    }

    fn update_code(&mut self, code: Vec<u8>, hash: Option<CryptoHash>) {
        self.contract = vec![CodeWrapper(Arc::new(ContractCode::new(code, hash)))];
    }

    fn push_code(&mut self, code: Vec<u8>, hash: Option<CryptoHash>) {
        self.contract
            .push(CodeWrapper(Arc::new(ContractCode::new(code, hash))));
    }

    fn pop_code(&mut self) {
        self.contract.pop();
    }

    pub fn get_version(&self) -> Result<String, GetVersionError<VMRunnerError>> {
        let env = Fixed {
            signer_account_id: "aurora".parse().unwrap(),
            current_account_id: "aurora".parse().unwrap(),
            predecessor_account_id: "aurora".parse().unwrap(),
            block_height: 0,
            block_timestamp: Timestamp::new(0),
            attached_deposit: 1,
            random_seed: H256::random(),
            prepaid_gas: NearGas::new(300_000_000_000_000),
            used_gas: NearGas::new(0),
        };
        let out = self
            .call("get_version", vec![], Arc::new([]), &env, &mut Nop)
            .map_err(GetVersionError::Inner)?;
        let data = out
            .return_data
            .as_value()
            .ok_or(GetVersionError::UnexpectedResponse)?;
        Ok(str::from_utf8(&data)?.trim_end().to_string())
    }

    fn call(
        &self,
        method: &str,
        input: Vec<u8>,
        promise_results: Arc<[PromiseResult]>,
        env: &impl Env,
        ext: &mut (impl External + Send),
    ) -> Result<VMOutcome, VMRunnerError> {
        let Some(wrapper) = self.contract.last() else {
            return Err(VMRunnerError::ContractCodeNotPresent);
        };

        let current_account_id = env
            .current_account_id()
            .to_string()
            .parse::<AccountId>()
            .expect("incompatible account id");
        let signer_account_id = env
            .signer_account_id()
            .to_string()
            .parse::<AccountId>()
            .expect("incompatible account id");
        let predecessor_account_id = env
            .predecessor_account_id()
            .to_string()
            .parse::<AccountId>()
            .expect("incompatible account id");
        let storage_usage =
            100 + u64::try_from(wrapper.0.code().len()).expect("usize must fit in 64");
        let ctx = VMContext {
            current_account_id,
            signer_account_id,
            signer_account_pk: vec![],
            predecessor_account_id,
            input,
            promise_results,
            block_height: env.block_height(),
            block_timestamp: env.block_timestamp().nanos(),
            epoch_height: 0,
            account_balance: 10u128.pow(25),
            account_locked_balance: 0,
            storage_usage,
            attached_deposit: env.attached_deposit(),
            prepaid_gas: env.prepaid_gas().as_u64(),
            random_seed: env.random_seed().0.to_vec(),
            output_data_receivers: vec![],
            view_config: None,
        };

        let contract = near_vm_runner::prepare(
            &*wrapper,
            self.runtime_config.wasm_config.clone(),
            Some(&self.cache),
            ctx.make_gas_counter(&self.runtime_config.wasm_config),
            method,
        );

        near_vm_runner::run(contract, ext, &ctx, self.runtime_config.fees.clone())
    }
}

#[derive(Clone)]
struct SimpleContractRuntimeCache {
    inner: Arc<Mutex<LruCache<CryptoHash, CompiledContractInfo>>>,
}

impl ContractRuntimeCache for SimpleContractRuntimeCache {
    fn handle(&self) -> Box<dyn ContractRuntimeCache> {
        Box::new(self.clone())
    }

    fn put(&self, key: &CryptoHash, value: CompiledContractInfo) -> std::io::Result<()> {
        self.inner.lock().unwrap().put(*key, value);
        Ok(())
    }

    fn get(&self, key: &CryptoHash) -> std::io::Result<Option<CompiledContractInfo>> {
        Ok(self.inner.lock().unwrap().get(key).cloned())
    }
}

impl AbstractContractRunner for ContractRunner {
    type Error = VMRunnerError;

    fn call_contract<E, I>(
        &self,
        method: &str,
        promise_data: Vec<Option<Vec<u8>>>,
        env: &E,
        io: I,
    ) -> Result<Option<Vec<u8>>, Self::Error>
    where
        E: Env,
        I: IO + Send,
        I::StorageValue: AsRef<[u8]>,
    {
        let promise_results = promise_data
            .iter()
            .cloned()
            .map(|data| data.map_or(PromiseResult::Failed, PromiseResult::Successful))
            .collect::<Vec<_>>()
            .into();

        let input = io.read_input().as_ref().to_vec();
        let mut ext = EngineStateVMAccess {
            io,
            action_log: vec![],
        };

        let vm_outcome = self.call(method, input, promise_results, env, &mut ext)?;
        let output = vm_outcome.return_data.as_value();
        if let Some(data) = &output {
            ext.io.return_output(data);
        }
        Ok(output)
    }
}

mod loader {
    use std::{
        collections::BTreeMap,
        fmt, fs, io,
        ops::Deref,
        path::PathBuf,
        str,
        sync::{Arc, Mutex, RwLock},
        time::Duration,
    };

    use aurora_refiner_types::source_config::ContractSource;
    use engine_standalone_storage::Storage;
    use near_jsonrpc_client::{
        JsonRpcClient, NEAR_TESTNET_RPC_URL,
        errors::JsonRpcError,
        methods::query::{RpcQueryError, RpcQueryRequest},
    };
    use near_jsonrpc_primitives::types::query::QueryResponseKind;
    use near_primitives_core::hash::CryptoHash;
    use tokio::time::Instant;

    use crate::{fetch_contract, storage_ext};

    use super::{ContractRunner, GetVersionError};

    async fn version(
        height: u64,
        mainnet: bool,
    ) -> Result<String, GetVersionError<JsonRpcError<RpcQueryError>>> {
        let url = if mainnet {
            "https://archival-rpc.mainnet.near.org"
        } else {
            NEAR_TESTNET_RPC_URL
        };
        let client = JsonRpcClient::connect(url);
        let request = serde_json::from_value::<RpcQueryRequest>(serde_json::json!({
            "request_type": "call_function",
            "block_id": height,
            "account_id": "aurora",
            "method_name": "get_version",
            "args_base64": "",
        }))
        .expect("Format query request");
        let result = tokio::time::timeout(Duration::from_secs(4), client.call(request))
            .await?
            .map_err(GetVersionError::Inner)?;
        match result.kind {
            QueryResponseKind::CallResult(r) => {
                Ok(str::from_utf8(&r.result)?.trim_end().to_string())
            }
            _ => Err(GetVersionError::UnexpectedResponse),
        }
    }

    struct VersionRequest {
        last_response: Option<Instant>,
        backoff: Duration,
    }

    impl Default for VersionRequest {
        fn default() -> Self {
            VersionRequest {
                last_response: None,
                backoff: Self::DEFAULT_DELAY,
            }
        }
    }

    impl VersionRequest {
        const DEFAULT_DELAY: Duration = Duration::from_secs(2);
        const EXPONENT: u32 = 2;

        async fn run(
            &mut self,
            height: u64,
        ) -> Result<String, GetVersionError<JsonRpcError<RpcQueryError>>> {
            loop {
                if let Some(last) = self.last_response {
                    tokio::time::sleep_until(last + self.backoff).await;
                } else {
                    tokio::time::sleep(self.backoff).await;
                }
                let res = version(height, true).await;
                self.last_response = Some(Instant::now());
                println!("{} -> {:?}, {:?}", height, res, self.backoff);
                if res.is_ok() {
                    self.backoff = Self::DEFAULT_DELAY;
                } else if let Err(err) = &res {
                    println!("{err}");
                    if err.to_string().contains("rate limit") {
                        self.backoff = (self.backoff * Self::EXPONENT).min(Duration::from_secs(60));
                        continue;
                    }
                }
                break res;
            }
        }
    }

    fn load_from_file(
        version: &str,
        override_prefix: Option<PathBuf>,
    ) -> io::Result<(Vec<u8>, Option<CryptoHash>)> {
        let prefix = override_prefix.clone().unwrap_or_else(|| "etc/res".into());
        let path = prefix.join(format!("aurora-engine-{}.wasm", version));
        fs::read(&path)
            .map(|code| (code, None))
            .map_err(|e| {
                let err = format!("Failed to read `{}`: {e}", path.display());
                io::Error::new(e.kind(), err)
            })
            .or_else(|err| {
                if override_prefix.is_none() {
                    // tests are run from the crate root, not from workspace root
                    load_from_file(version, Some(PathBuf::from("../etc/res")))
                } else {
                    Err(err)
                }
            })
    }

    #[derive(Clone)]
    struct VersionMap(BTreeMap<u64, String>);

    impl Default for VersionMap {
        fn default() -> Self {
            Self(
                [
                    (134229098, "3.7.0".to_owned()),
                    (143772514, "3.9.0".to_owned()),
                    (154664694, "3.9.1".to_owned()),
                    (159429079, "3.9.2".to_owned()),
                ]
                .into_iter()
                .collect(),
            )
        }
    }

    impl fmt::Display for VersionMap {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for (height, version) in &self.0 {
                writeln!(f, "{height} -> {version}")?;
            }

            Ok(())
        }
    }

    impl VersionMap {
        fn version_at_height(&self, height: u64) -> &str {
            self.0
                .iter()
                .take_while(|(x, _)| **x <= height)
                .last()
                .map(|(_, x)| x.as_ref())
                .unwrap_or_else(|| "3.6.4")
        }

        async fn populate(&mut self) {
            let mut req = VersionRequest::default();
            let (mut initial_height, mut current_version) =
                self.0.last_key_value().map(|(h, v)| (*h, v)).unwrap();
            while let (next_height, Some(next_version)) =
                Self::populate_next(&mut req, initial_height, current_version).await
            {
                initial_height = next_height;
                current_version = &*self.0.entry(next_height).or_insert(next_version);
            }
        }

        async fn populate_next(
            req: &mut VersionRequest,
            initial_height: u64,
            current_version: &str,
        ) -> (u64, Option<String>) {
            // initial step is 2^15 blocks,
            // will return back if overrun
            let mut step = 15;

            // the supposed next version, it is not final, might be overrun
            // `None` means out of range
            let mut next_version = loop {
                match req.run(initial_height + (1 << step)).await {
                    Ok(version) if version.ne(current_version) => {
                        break Some(version);
                    }
                    Err(err) if err.out_of_range() => {
                        break None;
                    }
                    Ok(_) => {
                        // go further
                        step += 1;
                    }
                    Err(_) => {
                        // TODO(vlad): limit retry
                    }
                }
            };

            step -= 1;
            let mut offset = 1 << step;
            let mut overrun;
            loop {
                match req.run(initial_height + offset).await {
                    Ok(version) if version.ne(current_version) => {
                        next_version = Some(version);
                        overrun = true;
                    }
                    Err(err) if err.out_of_range() => {
                        next_version = None;
                        overrun = true;
                    }
                    Ok(_) => {
                        overrun = false;
                    }
                    Err(_) => {
                        continue;
                    }
                }
                if step == 0 {
                    if !overrun {
                        offset += 1;
                    }
                    break;
                } else {
                    step -= 1;
                    if overrun {
                        offset -= 1 << step;
                    } else {
                        offset += 1 << step;
                    }
                }
            }

            (initial_height + offset, next_version)
        }
    }

    #[cfg(test)]
    mod tests_version_map {
        use super::{VersionMap, VersionRequest};

        #[test]
        fn version_map() {
            let map = VersionMap::default();

            assert_eq!(map.version_at_height(134229097), "3.6.4");
            assert_eq!(map.version_at_height(134229098), "3.7.0");
            assert_eq!(map.version_at_height(134229099), "3.7.0");

            assert_eq!(map.version_at_height(143772513), "3.7.0");
            assert_eq!(map.version_at_height(143772514), "3.9.0");
            assert_eq!(map.version_at_height(143772515), "3.9.0");

            assert_eq!(map.version_at_height(154664693), "3.9.0");
            assert_eq!(map.version_at_height(154664694), "3.9.1");
            assert_eq!(map.version_at_height(154664695), "3.9.1");

            assert_eq!(map.version_at_height(159429078), "3.9.1");
            assert_eq!(map.version_at_height(159429079), "3.9.2");
            assert_eq!(map.version_at_height(159429080), "3.9.2");
        }

        #[tokio::test]
        async fn out_of_range() {
            let mut req = VersionRequest::default();
            match req.run(200_000_000).await {
                Ok(_) => {}
                Err(err) if err.out_of_range() => {}
                Err(err) => panic!("unexpected error: {err}"),
            }
        }

        #[ignore = "rate limit for RPC is too strict, the test takes too long"]
        #[tokio::test]
        async fn version_map_rpc() {
            let map = VersionMap::default();
            let mut req = VersionRequest::default();

            for height in [
                (134229097..).take(3),
                (143772513..).take(3),
                (154664692..).take(3),
                (159429077..).take(3),
            ]
            .into_iter()
            .flatten()
            {
                let actual = req.run(height).await.unwrap();
                let expected = map.version_at_height(height);
                assert_eq!(actual, expected, "{height}");
            }
        }

        #[ignore = "rate limit for RPC is too strict, the test takes too long"]
        #[tokio::test]
        async fn version_map_populate() {
            let mut map = VersionMap::default();
            map.populate().await;
            println!("{map}");
        }
    }

    pub struct SeqAccessContractCache {
        current: ContractRunner,
    }

    impl SeqAccessContractCache {
        pub fn new_version(version: &str) -> io::Result<Self> {
            let (contract_bytes, contract_hash) = load_from_file(version, None)?;
            let current = ContractRunner::new_mainnet(contract_bytes, contract_hash);

            Ok(SeqAccessContractCache { current })
        }

        // TODO(vlad): workout error
        pub async fn initialize(
            height: u64,
            prefix: Option<PathBuf>,
            mainnet: bool,
        ) -> io::Result<Self> {
            let contract_version = version(height, mainnet)
                .await
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
            let (contract_bytes, contract_hash) = load_from_file(&contract_version, prefix)?;
            let current = ContractRunner::new_mainnet(contract_bytes, contract_hash);
            Ok(SeqAccessContractCache { current })
        }

        pub fn runner(&self) -> &ContractRunner {
            &self.current
        }

        pub fn update(
            &mut self,
            storage: &Storage,
            code: &[u8],
            hash: Option<CryptoHash>,
            block_height: u64,
            tx_pos: u16,
        ) {
            self.current.push_code(code.to_vec(), hash);
            let version = self.current.get_version().unwrap();
            self.current.pop_code();

            match storage_ext::get_contract(storage, block_height, tx_pos, &version) {
                Ok(Some(bytes)) => {
                    self.current.update_code(bytes, None);
                    return;
                }
                Ok(None) => { /* fallthrough to file load */ }
                Err(err) => {
                    tracing::error!(
                        err = format!("{err:?}"),
                        height = block_height,
                        "failed to get a contract",
                    );
                }
            }
            match load_from_file(&version, None) {
                Ok((code, hash)) => {
                    self.current.update_code(code, hash);
                }
                Err(err) => {
                    tracing::error!(
                        err = format!("{err:?}"),
                        height = block_height,
                        tx_pos = tx_pos,
                        new_version = &version,
                        "Failed to load contract. Send the contract via nats as soon as possible. Restart from {block_height}:{tx_pos}."
                    );
                }
            }
        }
    }

    #[derive(Clone)]
    pub struct RandomAccessContractCache {
        version_map: VersionMap,
        inner: CacheInner,
    }

    impl RandomAccessContractCache {
        pub fn new(link: Option<ContractSource>) -> Self {
            RandomAccessContractCache {
                version_map: VersionMap::default(),
                inner: CacheInner::new(link),
            }
        }
    }

    #[derive(Clone)]
    struct CacheInner {
        pool: Arc<Mutex<Vec<ContractRunner>>>,
        link: Option<ContractSource>,
    }

    impl CacheInner {
        pub fn new(link: Option<ContractSource>) -> Self {
            CacheInner {
                pool: Default::default(),
                link,
            }
        }
    }

    impl CacheInner {
        // if the code is not available, return None and fail the RPC
        // in this case add a task to fetch the code from nats
        async fn take_code(
            &self,
            storage: &RwLock<Storage>,
            block_height: u64,
            tx_pos: u16,
            version: &str,
        ) -> (Vec<u8>, Option<CryptoHash>) {
            {
                let storage_lock = storage.read().expect("storage must not panic");
                match storage_ext::get_contract(&storage_lock, block_height, tx_pos, version) {
                    Ok(Some(bytes)) => return (bytes, None),
                    Ok(None) => { /* fallthrough to file load */ }
                    Err(err) => {
                        tracing::error!(
                            err = format!("{err:?}"),
                            height = block_height,
                            "failed to get a contract",
                        );
                    }
                }
            }

            match load_from_file(version, None) {
                Ok(v) => {
                    return v;
                }
                Err(err) => {
                    tracing::error!(
                        err = format!("{err:?}"),
                        height = block_height,
                        new_version = version,
                        "Failed to load contract from file for RPC. Try to download."
                    );
                }
            }

            if let Some(link) = &self.link {
                if let Some(code) = fetch_contract::fetch_and_store_contract(
                    storage,
                    link,
                    version,
                    block_height,
                    tx_pos,
                )
                .await
                {
                    return (code, None);
                }
            }
            tracing::error!(
                height = block_height,
                tx_pos = tx_pos,
                new_version = version,
                "Failed to fetch contract from contract source. Will use fallback."
            );

            // last resort,
            // return something bundled, if RPC fails, so be it
            load_from_file("3.9.0", None).expect("cannot load fallback")
        }

        async fn take_runner(
            &self,
            storage: &RwLock<Storage>,
            block_height: u64,
            tx_pos: u16,
            version: &str,
        ) -> ContractRunner {
            let (code, hash) = self.take_code(storage, block_height, tx_pos, version).await;
            let mut runner = self
                .pool
                .lock()
                .expect("poisoned")
                .pop()
                .unwrap_or_else(|| ContractRunner::new_empty());
            runner.update_code(code, hash);
            runner
        }

        fn reuse(&self, runner: ContractRunner) {
            self.pool.lock().expect("poisoned").push(runner);
        }
    }

    impl RandomAccessContractCache {
        /// Warning: this function may take a long time to complete.
        /// Need to use RPC server with mild rate limit.
        pub async fn populate_map(&mut self) {
            self.version_map.populate().await;
        }

        pub async fn take_runner<'a>(
            &'a self,
            storage: &RwLock<Storage>,
            height: u64,
            tx_pos: u16,
        ) -> ReusableContractRunner<'a> {
            let version = self.version_map.version_at_height(height);
            let runner = self
                .inner
                .take_runner(storage, height, tx_pos, version)
                .await;
            ReusableContractRunner {
                cache: self,
                runner: Some(runner),
            }
        }

        fn reuse(&self, runner: ContractRunner) {
            self.inner.reuse(runner);
        }
    }

    pub struct ReusableContractRunner<'a> {
        cache: &'a RandomAccessContractCache,
        runner: Option<ContractRunner>,
    }

    impl Drop for ReusableContractRunner<'_> {
        fn drop(&mut self) {
            if let Some(runner) = self.runner.take() {
                self.cache.reuse(runner);
            }
        }
    }

    impl Deref for ReusableContractRunner<'_> {
        type Target = ContractRunner;

        fn deref(&self) -> &Self::Target {
            self.runner.as_ref().expect("runner is present")
        }
    }
}
pub use self::loader::{RandomAccessContractCache, SeqAccessContractCache};
