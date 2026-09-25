use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::inner_block::InnerNearBlock;
use tokio::sync::{broadcast, mpsc};

use crate::config::NearcoreConfig;

type BlockSender = mpsc::Sender<BlockWithMetadata<InnerNearBlock, ()>>;

/// Spawns a task that reads blocks from the nearcore indexer and sends converted blocks to the
/// channel. Returns the block receiver and a task handle that reports fatal input errors.
/// A fatal error also broadcasts the shutdown signal, so the rest of the application stops.
pub async fn get_nearcore_stream(
    block_height: u64,
    config: &NearcoreConfig,
    shutdown_tx: broadcast::Sender<()>,
) -> anyhow::Result<(
    mpsc::Receiver<BlockWithMetadata<InnerNearBlock, ()>>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
)> {
    tracing::info!(
        "get_nearcore_stream: starting nearcore stream, block_height: {block_height:?}..."
    );

    let (sender, receiver) = mpsc::channel(1000);
    let shutdown_rx = shutdown_tx.subscribe();

    let indexer_config = near_indexer::IndexerConfig {
        home_dir: std::path::PathBuf::from(&config.path),
        sync_mode: near_indexer::SyncModeEnum::BlockHeight(block_height),
        await_for_node_synced: near_indexer::AwaitForNodeSyncedEnum::StreamWhileSyncing,
        finality: near_indexer::near_primitives::types::Finality::Final,
        validate_genesis: true,
        skip_broken_blocks: false,
    };
    let indexer = near_indexer::Indexer::new(indexer_config).await?;
    tracing::info!("get_nearcore_stream: nearcore indexer started");

    let task_handle = tokio::spawn(async move {
        // Regular NEAR indexer process starts here
        let stream = indexer.streamer();
        tracing::info!("get_nearcore_stream: nearcore stream started");

        let result = forward_blocks(stream, sender, shutdown_rx).await;

        if let Err(err) = &result {
            tracing::error!("get_nearcore_stream: fatal error: {err}");
            let _ = shutdown_tx.send(());
        }

        result
    });

    tracing::info!("get_nearcore_stream: nearcore stream finished");

    Ok((receiver, task_handle))
}

/// Converts and forwards blocks until a shutdown signal or a fatal input error.
///
/// The shutdown signal is checked first: the refiner drops its input receiver after receiving
/// it, so a failed send at that point is part of the shutdown rather than an input failure.
async fn forward_blocks(
    mut stream: mpsc::Receiver<near_indexer::StreamerMessage>,
    sender: BlockSender,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    loop {
        let block = tokio::select! {
            biased;
            _ = shutdown_rx.recv() => break,
            block = stream.recv() => block
                .ok_or_else(|| anyhow::anyhow!("Nearcore block stream was closed"))?,
        };
        let block = InnerNearBlock::try_from(block)
            .map_err(|err| anyhow::anyhow!("Failed to convert nearcore block: {err}"))?;

        tokio::select! {
            biased;
            _ = shutdown_rx.recv() => break,
            result = sender.send(BlockWithMetadata::new(block, ())) => result
                .map_err(|_| anyhow::anyhow!("Refiner input receiver was dropped"))?,
        }
    }

    // Explicitly close the channel, so the tx side should stop sending blocks
    stream.close();
    tracing::info!("get_nearcore_stream: Received shutdown signal");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const STREAMER_MESSAGE: &str = include_str!(
        "../../../refiner-types/tests/res/streamer_message_190534818_branch_remove_custom_indexer.json"
    );

    fn streamer_message() -> near_indexer::StreamerMessage {
        serde_json::from_str(STREAMER_MESSAGE).unwrap()
    }

    #[tokio::test]
    async fn forwards_blocks_until_shutdown() {
        let (stream_tx, stream_rx) = mpsc::channel(1);
        let (sender, mut receiver) = mpsc::channel(1);
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        stream_tx.send(streamer_message()).await.unwrap();

        let task = tokio::spawn(forward_blocks(stream_rx, sender, shutdown_rx));

        let block = receiver.recv().await.unwrap();
        assert_eq!(block.block.block.header.height, 190_534_818);
        shutdown_tx.send(()).unwrap();
        assert!(task.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn dropped_receiver_during_shutdown_is_not_an_error() {
        let (stream_tx, stream_rx) = mpsc::channel(1);
        let (sender, receiver) = mpsc::channel(1);
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        // Fill the refiner input, so forwarding the next block waits for the refiner.
        let block = InnerNearBlock::try_from(streamer_message()).unwrap();
        sender
            .send(BlockWithMetadata::new(block, ()))
            .await
            .unwrap();
        stream_tx.send(streamer_message()).await.unwrap();

        let task = tokio::spawn(forward_blocks(stream_rx, sender, shutdown_rx));
        tokio::task::yield_now().await;

        // The refiner receives the shutdown signal and then drops its input receiver.
        shutdown_tx.send(()).unwrap();
        drop(receiver);

        assert!(task.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn reports_dropped_refiner_receiver() {
        let (stream_tx, stream_rx) = mpsc::channel(1);
        let (sender, receiver) = mpsc::channel(1);
        let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);
        stream_tx.send(streamer_message()).await.unwrap();
        drop(receiver);

        let err = forward_blocks(stream_rx, sender, shutdown_rx)
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "Refiner input receiver was dropped");
    }

    #[tokio::test]
    async fn reports_closed_nearcore_stream() {
        let (stream_tx, stream_rx) = mpsc::channel(1);
        let (sender, _receiver) = mpsc::channel(1);
        let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);
        drop(stream_tx);

        let err = forward_blocks(stream_rx, sender, shutdown_rx)
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "Nearcore block stream was closed");
    }
}
