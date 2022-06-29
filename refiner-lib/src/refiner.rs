use crate::near_stream::NearStream;
use aurora_refiner_types::aurora_block::AuroraBlock;
use aurora_refiner_types::near_block::NEARBlock;
use aurora_standalone_engine::EngineContext;
use tracing::warn;

// TODO: Move to config file
const AURORA: &'static str = "aurora";

pub async fn run_refiner(
    chain_id: u64,
    mut input: tokio::sync::mpsc::Receiver<NEARBlock>,
    output: tokio::sync::mpsc::Sender<AuroraBlock>,
    last_block: Option<u64>,
) {
    // TODO: Add storage path to config file
    let ctx = EngineContext::new("", AURORA.parse().unwrap(), chain_id).unwrap();
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
