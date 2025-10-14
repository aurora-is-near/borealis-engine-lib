use aurora_engine_modexp::AuroraModExp;
use aurora_engine_types::H256;
use engine_standalone_storage::{
    Storage,
    sync::{self, TransactionIncludedOutcome},
};
use engine_standalone_tracing::{TraceKind, types::call_tracer::CallTracer};
use tokio::sync::RwLock;

use crate::runner;

pub struct DebugTraceTransactionRequest {
    pub tx_hash: H256,
}

impl DebugTraceTransactionRequest {
    pub fn from_json_value(body: serde_json::Value) -> Option<Self> {
        let tx_hash = body.get("params")?.get(0)?.as_str()?;
        let tx_hash = tx_hash.strip_prefix("0x").unwrap_or(tx_hash);
        let bytes = hex::decode(tx_hash).ok()?;
        if bytes.len() != 32 {
            return None;
        }
        Some(Self {
            tx_hash: H256::from_slice(&bytes),
        })
    }
}

pub async fn trace_transaction(
    storage: &RwLock<Storage>,
    cache: &runner::RandomAccessContractCache,
    tx_hash: H256,
) -> Result<(CallTracer, TransactionIncludedOutcome), engine_standalone_storage::Error> {
    let storage_lock = storage.read().await;
    let tx_msg = storage_lock.get_transaction_data(tx_hash)?;
    let height = storage_lock.get_block_height_by_hash(tx_msg.block_hash)?;
    drop(storage_lock);
    let runner = cache.take_runner(storage, height, tx_msg.position).await;
    let storage_lock = storage.read().await;
    let mut outcome = sync::execute_transaction_message::<AuroraModExp, runner::ContractRunner>(
        &storage_lock,
        &*runner,
        tx_msg,
        Some(TraceKind::CallFrame),
    )?;
    drop(storage_lock);

    let tracer = outcome.call_tracer.take().unwrap();
    Ok((tracer, outcome))
}
