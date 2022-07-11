use crate::near_stream::NearStream;
use aurora_engine_types::account_id::AccountId;
use aurora_refiner_types::aurora_block::AuroraBlock;
use aurora_refiner_types::near_block::NEARBlock;
use aurora_standalone_engine::EngineContext;
use std::fmt::Debug;
use std::path::Path;

#[derive(Debug)]
pub struct BlockWithMetadata<B: Debug, M: Debug + Clone> {
    pub block: B,
    pub metadata: M,
}

impl<B: Debug, M: Debug + Clone> BlockWithMetadata<B, M> {
    pub fn new(block: B, metadata: M) -> Self {
        Self { block, metadata }
    }
}

pub async fn run_refiner<P: AsRef<Path>, M: Debug + Clone>(
    chain_id: u64,
    engine_storage_path: P,
    engine_account_id: AccountId,
    mut input: tokio::sync::mpsc::Receiver<BlockWithMetadata<NEARBlock, M>>,
    output: tokio::sync::mpsc::Sender<BlockWithMetadata<AuroraBlock, M>>,
    last_block: Option<u64>,
) {
    let ctx = EngineContext::new(engine_storage_path, engine_account_id, chain_id).unwrap();
    let mut stream = NearStream::new(chain_id, last_block, ctx);

    while let Some(message) = input.recv().await {
        let BlockWithMetadata { block, metadata } = message;
        for block in stream.next_block(block) {
            // Unwrapping here, since it is better to crash the refiner than to make progress missing blocks.
            output
                .send(BlockWithMetadata::new(block, metadata.clone()))
                .await
                .expect("Failed to send output message");
        }
    }

    tracing::warn!("Input stream was closed.")
}
