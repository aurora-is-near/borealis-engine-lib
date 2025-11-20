use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::near_block::NEARBlock;

use crate::config::NearcoreConfig;

pub async fn get_nearcore_stream(
    block_height: u64,
    config: &NearcoreConfig,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<(
    tokio::sync::mpsc::Receiver<BlockWithMetadata<NEARBlock, ()>>,
    tokio::task::JoinHandle<()>,
)> {
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
    let near_config = indexer_config.load_near_config()?;
    let near_node =
        near_indexer::Indexer::start_near_node(&indexer_config, near_config.clone()).await?;
    let indexer = near_indexer::Indexer::from_near_node(indexer_config, near_config, &near_node);
    tracing::info!("get_nearcore_stream: nearcore indexer started");

    let task_handle = tokio::spawn(async move {
        // Regular NEAR indexer process starts here
        let mut stream = indexer.streamer();
        tracing::info!("get_nearcore_stream: nearcore stream started");
        loop {
            tokio::select! {
                Some(block) = stream.recv() => {
                    sender
                        .send(BlockWithMetadata::new(
                            aurora_refiner_types::conversion::nearcore::convert(block),
                            ())
                        )
                        .await
                        .expect("Failed to send block to the channel from nearcore stream");
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

    tracing::info!("get_nearcore_stream: nearcore stream finished");

    Ok((receiver, task_handle))
}
