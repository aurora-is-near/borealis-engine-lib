use aurora_engine_modexp::AuroraModExp;
use aurora_engine_types::H256;
use engine_standalone_storage::{
    Storage,
    sync::{self, TransactionIncludedOutcome},
};
use engine_standalone_tracing::{TraceKind, types::call_tracer::CallTracer};

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

pub fn trace_transaction(
    storage: &Storage,
    cache: &runner::Cache,
    tx_hash: H256,
) -> Result<(CallTracer, TransactionIncludedOutcome), engine_standalone_storage::Error> {
    let tx_msg = storage.get_transaction_data(tx_hash)?;
    let height = storage.get_block_height_by_hash(tx_msg.block_hash)?;
    let mut outcome = cache.with_runner(height, |runner| {
        sync::execute_transaction_message::<AuroraModExp, _>(
            storage,
            runner,
            tx_msg,
            Some(TraceKind::CallFrame),
        )
    })?;

    let tracer = outcome.call_tracer.take().unwrap();
    Ok((tracer, outcome))
}
