use aurora_engine_modexp::AuroraModExp;
use aurora_engine_types::H256;
use engine_standalone_storage::{
    sync::{self, TransactionIncludedOutcome},
    Storage,
};
use engine_standalone_tracing::{sputnik, types::call_tracer::CallTracer};

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
    tx_hash: H256,
) -> Result<(CallTracer, TransactionIncludedOutcome), engine_standalone_storage::Error> {
    let tx_msg = storage.get_transaction_data(tx_hash)?;
    let mut listener = CallTracer::default();
    let outcome = sputnik::traced_call(&mut listener, || {
        sync::execute_transaction_message::<AuroraModExp>(storage, tx_msg)
    })?;
    Ok((listener, outcome))
}
