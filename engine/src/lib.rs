use aurora_engine_modexp::ModExpAlgorithm;
use aurora_engine_types::{account_id::AccountId, H256};
use aurora_refiner_types::{near_block::NEARBlock, near_primitives::hash::CryptoHash};
use engine_standalone_storage::{error, sync::TransactionIncludedOutcome, Storage};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;

pub mod sync;
#[cfg(test)]
mod tests;
pub mod tracing;
pub mod types;

pub struct EngineContext {
    pub storage: Storage,
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
        let chain_id = aurora_engine_types::types::u256_to_arr(&(chain_id.into()));
        Ok(Self {
            storage,
            engine_account_id,
            chain_id,
            data_id_mapping: lru::LruCache::new(NonZeroUsize::new(1000).unwrap()),
        })
    }
}

pub fn consume_near_block<M: ModExpAlgorithm>(
    block: &NEARBlock,
    context: &mut EngineContext,
    outcomes: Option<&mut HashMap<H256, TransactionIncludedOutcome>>,
) -> Result<(), error::Error> {
    sync::consume_near_block::<M>(
        &mut context.storage,
        block,
        &mut context.data_id_mapping,
        &context.engine_account_id,
        context.chain_id,
        outcomes,
    )
}
