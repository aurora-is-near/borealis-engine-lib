use aurora_engine_types::{account_id::AccountId, H256};
use aurora_refiner_types::{near_block::NEARBlock, near_primitives::hash::CryptoHash};
use engine_standalone_storage::{error, sync::TransactionIncludedOutcome, Storage};
use std::collections::HashMap;
use std::path::Path;

pub mod sync;
#[cfg(test)]
mod tests;
pub mod types;

pub struct EngineContext {
    storage: Storage,
    engine_account_id: AccountId,
    data_id_mapping: lru::LruCache<CryptoHash, Option<Vec<u8>>>,
}

impl EngineContext {
    pub fn new<P: AsRef<Path>>(
        storage_path: P,
        engine_account_id: AccountId,
    ) -> Result<Self, error::Error> {
        let storage = Storage::open(storage_path)?;
        Ok(Self {
            storage,
            engine_account_id,
            data_id_mapping: lru::LruCache::new(1000),
        })
    }
}

pub fn consume_near_block(
    block: &NEARBlock,
    context: &mut EngineContext,
    outcomes: Option<&mut HashMap<H256, TransactionIncludedOutcome>>,
) -> Result<(), error::Error> {
    sync::consume_near_block(
        &mut context.storage,
        block,
        &mut context.data_id_mapping,
        &context.engine_account_id,
        outcomes,
    )
}
