use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::near_block::NEARBlock;

use crate::{
    config::NearcoreConfig,
    conversion::{ch_json, convert},
};

pub fn get_nearcore_stream(
    block_height: u64,
    config: &NearcoreConfig,
) -> tokio::sync::mpsc::Receiver<BlockWithMetadata<NEARBlock, ()>> {
    let (sender, receiver) = tokio::sync::mpsc::channel(1000);

    let indexer_config = near_indexer::IndexerConfig {
        home_dir: std::path::PathBuf::from(&config.path),
        sync_mode: near_indexer::SyncModeEnum::BlockHeight(block_height),
        await_for_node_synced: near_indexer::AwaitForNodeSyncedEnum::StreamWhileSyncing,
        finality: near_indexer::near_primitives::types::Finality::Final,
        validate_genesis: true,
    };

    // let indexer_config = construct_near_indexer_config(&pool, home_dir, args.clone()).await;
    let indexer = near_indexer::Indexer::new(indexer_config).expect("Failed to initiate Indexer");

    // Regular indexer process starts here
    let mut stream = indexer.streamer();

    tokio::spawn(async move {
        while let Some(block) = stream.recv().await {
            // TODO: Slow conversion between types. Fix
            sender
                .send(BlockWithMetadata::new(convert(ch_json(block)), ()))
                .await
                .unwrap();
        }
    });

    receiver
}
