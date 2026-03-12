use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::near_block::NEARBlock;
use near_lake_framework::{LakeBuilder, near_lake_primitives};

use crate::config::DataLakeConfig;

/// Spawns a task that reads blocks from the NEAR Data Lake stream and sends them to the channel.
/// The `shutdown_rx` is used to signal the task to stop.
/// Returns a channel to send NEAR blocks to the task and a handle to the task.
pub fn get_near_data_lake_stream(
    block_height: u64,
    config: &DataLakeConfig,
) -> (
    tokio::sync::mpsc::Receiver<BlockWithMetadata<NEARBlock, ()>>,
    tokio::task::JoinHandle<()>,
) {
    tracing::info!(
        "get_near_data_lake_stream: starting data lake stream, block_height: {block_height:?}..."
    );

    let lake = match config.network {
        crate::config::Network::Mainnet => LakeBuilder::default().mainnet(),
        crate::config::Network::Testnet => LakeBuilder::default().testnet(),
    }
    .start_block_height(block_height)
    .build()
    .expect("Failed to build Lake");

    let (sender, receiver) = tokio::sync::mpsc::channel(1000);

    let task_handle = tokio::spawn(async move {
        tracing::info!("get_near_data_lake_stream: data lake stream started");

        let context = DataLakeContext { sender };

        if let Err(err) = lake
            .run_with_context_async(
                |block, context: &DataLakeContext| {
                    let sender = context.sender.clone();
                    async move {
                        let block_with_meta = BlockWithMetadata::new(
                            aurora_refiner_types::conversion::data_lake::convert(
                                block.streamer_message().clone(),
                            ),
                            (),
                        );

                        sender
                            .send(block_with_meta)
                            .await
                            .expect("Failed to send block to channel from data lake stream");

                        Ok::<(), Box<dyn std::error::Error>>(())
                    }
                },
                &context,
            )
            .await
        {
            tracing::error!("get_near_data_lake_stream: data lake stream failed: {err}");
        }
    });

    tracing::info!("get_near_data_lake_stream: data lake stream finished");

    (receiver, task_handle)
}

struct DataLakeContext {
    sender: tokio::sync::mpsc::Sender<BlockWithMetadata<NEARBlock, ()>>,
}

impl near_lake_framework::LakeContextExt for DataLakeContext {
    fn execute_before_run(&self, _block: &mut near_lake_primitives::Block) {}

    fn execute_after_run(&self) {}
}
