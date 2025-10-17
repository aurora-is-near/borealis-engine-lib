use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, RwLock};

use aurora_engine_modexp::ModExpAlgorithm;
use aurora_engine_types::{H256, account_id::AccountId};
use aurora_refiner_types::{near_block::NEARBlock, near_primitives::hash::CryptoHash};
use engine_standalone_storage::{Storage, error, sync::TransactionIncludedOutcome};

use crate::runner::SeqAccessContractCache;

mod batch_tx_processing;
pub mod fetch_contract;
pub mod gas;
pub mod runner;
mod storage_ext;
pub mod sync;
#[cfg(test)]
mod tests;
pub mod tracing;

pub type SharedStorage = std::sync::Arc<RwLock<Storage>>;

pub struct EngineContext {
    pub storage: SharedStorage,
    pub engine_account_id: AccountId,
    chain_id: [u8; 32],
    data_id_mapping: lru::LruCache<CryptoHash, Option<Vec<u8>>>,
    contract: SeqAccessContractCache,
}

impl EngineContext {
    pub fn new<P: AsRef<Path>>(
        storage_path: P,
        contract: SeqAccessContractCache,
        engine_account_id: AccountId,
        chain_id: u64,
    ) -> Result<Self, error::Error> {
        let storage = Storage::open(storage_path)?;
        let storage = Arc::new(RwLock::new(storage));
        let chain_id = aurora_engine_types::types::u256_to_arr(&(chain_id.into()));
        Ok(Self {
            storage,
            engine_account_id,
            chain_id,
            data_id_mapping: lru::LruCache::new(NonZeroUsize::new(1000).unwrap()),
            contract,
        })
    }
}

pub async fn consume_near_block<M: ModExpAlgorithm>(
    block: &NEARBlock,
    context: &mut EngineContext,
    outcomes: Option<&mut HashMap<H256, TransactionIncludedOutcome>>,
) -> Result<(), error::Error> {
    let mut storage = context
        .storage
        .as_ref()
        .write()
        .expect("storage must not panic");
    sync::consume_near_block::<M>(
        &mut storage,
        &mut context.contract,
        block,
        &mut context.data_id_mapping,
        &context.engine_account_id,
        context.chain_id,
        outcomes,
    )
}
