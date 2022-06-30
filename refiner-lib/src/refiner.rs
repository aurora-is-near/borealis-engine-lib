use crate::near_stream::NearStream;
use aurora_engine_types::account_id::AccountId;
use aurora_refiner_types::aurora_block::AuroraBlock;
use aurora_refiner_types::near_block::NEARBlock;
use aurora_standalone_engine::EngineContext;
use std::path::Path;
use tracing::warn;

pub async fn run_refiner<P: AsRef<Path>>(
    chain_id: u64,
    engine_storage_path: P,
    engine_account_id: AccountId,
    mut input: tokio::sync::mpsc::Receiver<NEARBlock>,
    output: tokio::sync::mpsc::Sender<AuroraBlock>,
    last_block: Option<u64>,
) {
    let ctx = EngineContext::new(engine_storage_path, engine_account_id, chain_id).unwrap();
    let mut stream = NearStream::new(chain_id, last_block, ctx);

    while let Some(message) = input.recv().await {
        for block in stream.next_block(message) {
            // Unwrapping here, since it is better to crash the refiner than to make progress missing blocks.
            output
                .send(block)
                .await
                .expect("Failed to send output message");
        }
    }

    warn!("Input stream was closed.")
}
