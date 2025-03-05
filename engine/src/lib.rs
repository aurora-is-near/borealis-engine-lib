use aurora_engine_modexp::ModExpAlgorithm;
use aurora_engine_types::{H256, account_id::AccountId};
use aurora_refiner_types::{near_block::NEARBlock, near_primitives::hash::CryptoHash};
use engine_standalone_storage::{Storage, error, sync::TransactionIncludedOutcome};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;

mod batch_tx_processing;
pub mod gas;
pub mod sync;
#[cfg(test)]
mod tests;
pub mod tracing;

pub type SharedStorage = std::sync::Arc<tokio::sync::RwLock<Storage>>;

pub struct EngineContext {
    pub storage: SharedStorage,
    pub engine_account_id: AccountId,
    chain_id: [u8; 32],
    data_id_mapping: lru::LruCache<CryptoHash, Option<Vec<u8>>>,
}

impl EngineContext {
    pub fn new<P: AsRef<Path>>(
        storage_path: P,
        engine_account_id: AccountId,
        chain_id: u64,
    ) -> Result<Self, error::Error> {
        let storage = Storage::open(storage_path)?;
        let storage = std::sync::Arc::new(tokio::sync::RwLock::new(storage));
        let chain_id = aurora_engine_types::types::u256_to_arr(&(chain_id.into()));
        Ok(Self {
            storage,
            engine_account_id,
            chain_id,
            data_id_mapping: lru::LruCache::new(NonZeroUsize::new(1000).unwrap()),
        })
    }
}

pub async fn consume_near_block<M: ModExpAlgorithm>(
    block: &NEARBlock,
    context: &mut EngineContext,
    outcomes: Option<&mut HashMap<H256, TransactionIncludedOutcome>>,
) -> Result<(), error::Error> {
    let mut storage = context.storage.as_ref().write().await;
    sync::consume_near_block::<M>(
        &mut storage,
        block,
        &mut context.data_id_mapping,
        &context.engine_account_id,
        context.chain_id,
        outcomes,
    )
}
