use crate::near_stream::NearStream;
use crate::tx_hash_tracker;
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
    ctx: EngineContext,
    chain_id: u64,
    engine_account_id: AccountId,
    tx_storage_path: P,
    mut input: tokio::sync::mpsc::Receiver<BlockWithMetadata<NEARBlock, M>>,
    output: tokio::sync::mpsc::Sender<BlockWithMetadata<AuroraBlock, M>>,
    last_block: Option<u64>,
    stop_signal: &mut tokio::sync::broadcast::Receiver<()>,
) {
    let tx_tracker =
        tx_hash_tracker::TxHashTracker::new(tx_storage_path, last_block.unwrap_or_default())
            .expect("Failed to start transaction tracker");
    let mut stream = NearStream::new(chain_id, engine_account_id, last_block, ctx, tx_tracker);

    loop {
        tokio::select! {
            _ = stop_signal.recv() => {
                break
            }
            maybe_message = input.recv() => {
                if let Some(message) = maybe_message {
                    let BlockWithMetadata { block, metadata } = message;
                    for block in stream.next_block(&block).await {
                        // Unwrapping here, since it is better to crash the refiner than to make progress missing blocks.
                        output
                            .send(BlockWithMetadata::new(block, metadata.clone()))
                            .await
                            .map_err(|_| ())
                            .expect("Failed to send output message");
                    }
                } else {
                    tracing::warn!("Refiner input stream was closed.");
                    break
                }
            }
        }
    }
}
