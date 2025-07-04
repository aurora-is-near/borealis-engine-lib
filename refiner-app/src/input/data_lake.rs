use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::near_block::NEARBlock;
use near_lake_framework::LakeConfigBuilder;

use crate::config::DataLakeConfig;

/// Spawns a task that reads blocks from the NEAR Data Lake stream and sends them to the channel.
/// The `shutdown_rx` is used to signal the task to stop.
/// Returns a channel to send NEAR blocks to the task and a handle to the task.
pub fn get_near_data_lake_stream(
    block_height: u64,
    config: &DataLakeConfig,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> (
    tokio::sync::mpsc::Receiver<BlockWithMetadata<NEARBlock, ()>>,
    tokio::task::JoinHandle<()>,
) {
    tracing::info!(
        "get_near_data_lake_stream: starting data lake stream, block_height: {block_height:?}..."
    );

    let mut opts = LakeConfigBuilder::default();
    opts = match config.network {
        crate::config::Network::Mainnet => opts.mainnet(),
        crate::config::Network::Testnet => opts.testnet(),
    };
    let opts = opts
        .start_block_height(block_height)
        .build()
        .expect("Failed to build LakeConfig");

    let (sender, receiver) = tokio::sync::mpsc::channel(1000);

    let task_handle = tokio::spawn(async move {
        // Instantiate the NEAR Lake Framework Stream
        let (_, mut stream) = near_lake_framework::streamer(opts);
        loop {
            tokio::select! {
                Some(block) = stream.recv() => {
                    sender
                        .send(BlockWithMetadata::new(
                            aurora_refiner_types::conversion::data_lake::convert(block),
                            (),
                        ))
                        .await
                        .expect("Failed to send block to channel from data lake stream");
                }
                _ = shutdown_rx.recv() => {
                    // Explicitly close the channel, so the tx side should stop sending blocks
                    stream.close();
                    tracing::info!("get_near_data_lake_stream: Received shutdown signal");
                    break;
                }
            }
        }
    });

    (receiver, task_handle)
}
