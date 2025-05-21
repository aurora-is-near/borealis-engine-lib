use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::near_block::NEARBlock;

use crate::{
    config::NearcoreConfig,
    conversion::{ch_json, convert},
};

pub fn get_nearcore_stream(
    block_height: u64,
    config: &NearcoreConfig,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> (
    tokio::sync::mpsc::Receiver<BlockWithMetadata<NEARBlock, ()>>,
    tokio::task::JoinHandle<()>,
) {
    tracing::info!(
        "get_nearcore_stream: starting nearcore stream, block_height: {block_height:?}..."
    );

    let (sender, receiver) = tokio::sync::mpsc::channel(1000);

    let indexer_config = near_indexer::IndexerConfig {
        home_dir: std::path::PathBuf::from(&config.path),
        sync_mode: near_indexer::SyncModeEnum::BlockHeight(block_height),
        await_for_node_synced: near_indexer::AwaitForNodeSyncedEnum::StreamWhileSyncing,
        finality: near_indexer::near_primitives::types::Finality::Final,
        validate_genesis: true,
    };

    let indexer = near_indexer::Indexer::new(indexer_config).expect("Failed to initiate Indexer");

    let task_handle = tokio::task::spawn_local(async move {
        // Regular NEAR indexer process starts here
        let mut stream = indexer.streamer();
        loop {
            tokio::select! {
                Some(block) = stream.recv() => {
                    sender
                        .send(BlockWithMetadata::new(crate::conversion::conversion::nearcore::convert(block), ()))
                        .await
                        .expect("Failed to send block to channel from nearcore stream");
                }
                _ = shutdown_rx.recv() => {
                    // Explicitly close the channel, so the tx side should stop sending blocks
                    stream.close();
                    tracing::info!("get_nearcore_stream: Received shutdown signal");
                    break;
                }
            }
        }
    });

    (receiver, task_handle)
}
