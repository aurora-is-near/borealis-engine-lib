mod dump;

use std::path::PathBuf;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let indexer = near_indexer::Indexer::new(near_indexer::IndexerConfig {
        home_dir: PathBuf::from("storage"), // TODO: home_dir from configuration
        sync_mode: near_indexer::SyncModeEnum::FromInterruption,
        await_for_node_synced: near_indexer::AwaitForNodeSyncedEnum::StreamWhileSyncing,
    })
    .expect("Failed to initiate Indexer");

    let mut stream = indexer.streamer();
    while let Some(msg) = stream.recv().await {
        println!("{:?}", msg);
    }

    Ok(())
}
