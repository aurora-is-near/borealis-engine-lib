use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::inner_block::InnerNearBlock;
use near_lake_framework::LakeBuilder;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{broadcast, mpsc};

use crate::config::DataLakeConfig;

type BlockSender = mpsc::Sender<BlockWithMetadata<InnerNearBlock, ()>>;

/// Spawns a task that reads NEAR block JSON and sends converted blocks to the channel.
/// Returns the block receiver and a task handle that reports fatal input errors.
pub fn get_near_json_stream(
    block_height: u64,
    config: &DataLakeConfig,
    engine_account_id: &str,
    shutdown_tx: broadcast::Sender<()>,
) -> (
    mpsc::Receiver<BlockWithMetadata<InnerNearBlock, ()>>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    tracing::info!(
        "get_near_json_stream: starting block stream, block_height: {block_height:?}..."
    );

    let block_source = match config.network {
        crate::config::Network::Mainnet => LakeBuilder::default().mainnet(),
        crate::config::Network::Testnet => LakeBuilder::default().testnet(),
    }
    .start_block_height(block_height)
    .build()
    .expect("Failed to build NEAR block source");

    let (sender, receiver) = mpsc::channel(1000);
    let mut shutdown_rx = shutdown_tx.subscribe();
    let engine_account_id: Arc<str> = Arc::from(engine_account_id);

    let task_handle = tokio::spawn(async move {
        tracing::info!("get_near_json_stream: block stream started");

        let (fatal_error_tx, mut fatal_error_rx) = mpsc::unbounded_channel();
        let context = RawBlockContext {
            sender,
            fatal_error_tx,
            stopped: Arc::new(AtomicBool::new(false)),
            engine_account_id,
        };

        let source_run = block_source.run_raw_with_context_async(
            |block_bytes, context| {
                let sender = context.sender.clone();
                let fatal_error_tx = context.fatal_error_tx.clone();
                let stopped = Arc::clone(&context.stopped);
                let engine_account_id = Arc::clone(&context.engine_account_id);

                async move {
                    forward_block(
                        block_bytes,
                        &engine_account_id,
                        sender,
                        fatal_error_tx,
                        stopped,
                    )
                    .await
                }
            },
            &context,
        );

        let result = wait_for_input_end(source_run, &mut fatal_error_rx, &mut shutdown_rx).await;

        context.stopped.store(true, Ordering::Release);

        if let Err(err) = &result {
            tracing::error!("get_near_json_stream: fatal error: {err}");
            let _ = shutdown_tx.send(());
        }

        result
    });

    (receiver, task_handle)
}

/// Waits for the block source to finish, a fatal input error, or a shutdown signal.
///
/// The shutdown signal is checked first: the refiner drops its input receiver after receiving
/// it, so a failed sending reported at that point is part of the shutdown rather than an input
/// failure. A fatal error is checked before the source result, so it is never lost.
async fn wait_for_input_end<E: std::fmt::Display>(
    source_run: impl Future<Output = Result<(), E>>,
    fatal_error_rx: &mut mpsc::UnboundedReceiver<anyhow::Error>,
    shutdown_rx: &mut broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    tokio::select! {
        biased;
        _ = shutdown_rx.recv() => {
            tracing::info!("get_near_json_stream: received shutdown signal");
            Ok(())
        }
        Some(err) = fatal_error_rx.recv() => Err(err),
        result = source_run => result
            .map_err(|err| anyhow::anyhow!("NEAR block stream failed: {err}")),
    }
}

struct RawBlockContext {
    sender: BlockSender,
    fatal_error_tx: mpsc::UnboundedSender<anyhow::Error>,
    stopped: Arc<AtomicBool>,
    engine_account_id: Arc<str>,
}

impl<M> near_lake_framework::LakeContextExt<M> for RawBlockContext {
    fn execute_before_run(&self, _block: &mut M) {}

    fn execute_after_run(&self) {}
}

async fn forward_block(
    block_bytes: Vec<u8>,
    engine_account_id: &str,
    sender: BlockSender,
    fatal_error_tx: mpsc::UnboundedSender<anyhow::Error>,
    stopped: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    anyhow::ensure!(!stopped.load(Ordering::Acquire), "Refiner input stopped");

    let block = InnerNearBlock::from_bytes(&block_bytes)
        .map_err(Into::into)
        .and_then(|block: InnerNearBlock| {
            block.validate_for_engine(engine_account_id)?;
            Ok(block)
        })
        .inspect_err(|err: &anyhow::Error| {
            signal_fatal_error(
                &fatal_error_tx,
                &stopped,
                anyhow::anyhow!(
                    "Failed to deserialize NEAR block{}: {err}",
                    block_identity(&block_bytes)
                ),
            );
        })?;

    anyhow::ensure!(!stopped.load(Ordering::Acquire), "Refiner input stopped");

    sender
        .send(BlockWithMetadata::new(block, ()))
        .await
        .inspect_err(|_| {
            signal_fatal_error(
                &fatal_error_tx,
                &stopped,
                anyhow::anyhow!("Refiner input receiver was dropped"),
            );
        })
        .map_err(Into::into)
}

fn signal_fatal_error(
    fatal_error_tx: &mpsc::UnboundedSender<anyhow::Error>,
    stopped: &AtomicBool,
    err: anyhow::Error,
) {
    if !stopped.swap(true, Ordering::AcqRel) {
        let _ = fatal_error_tx.send(err);
    }
}

fn block_identity(block_bytes: &[u8]) -> String {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(block_bytes) else {
        return String::new();
    };
    let Some(header) = value.get("block").and_then(|block| block.get("header")) else {
        return String::new();
    };
    let height = header.get("height").and_then(serde_json::Value::as_u64);
    let hash = header.get("hash").and_then(serde_json::Value::as_str);

    match (height, hash) {
        (Some(height), Some(hash)) => format!(" at height {height} ({hash})"),
        (Some(height), None) => format!(" at height {height}"),
        (None, Some(hash)) => format!(" with hash {hash}"),
        (None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &[u8] =
        include_bytes!("../../../refiner-types/tests/res/block_190534818_branch_main.json");

    struct TestChannels {
        sender: BlockSender,
        receiver: mpsc::Receiver<BlockWithMetadata<InnerNearBlock, ()>>,
        fatal_error_tx: mpsc::UnboundedSender<anyhow::Error>,
        fatal_error_rx: mpsc::UnboundedReceiver<anyhow::Error>,
        stopped: Arc<AtomicBool>,
    }

    fn channels() -> TestChannels {
        let (sender, receiver) = mpsc::channel(1);
        let (fatal_error_tx, fatal_error_rx) = mpsc::unbounded_channel();
        TestChannels {
            sender,
            receiver,
            fatal_error_tx,
            fatal_error_rx,
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    #[tokio::test]
    async fn forwards_valid_block() {
        let TestChannels {
            sender,
            mut receiver,
            fatal_error_tx,
            mut fatal_error_rx,
            stopped,
        } = channels();

        assert!(
            forward_block(BLOCK.to_vec(), "aurora", sender, fatal_error_tx, stopped)
                .await
                .is_ok()
        );

        let block = receiver.recv().await.unwrap();
        assert_eq!(block.block.block.header.height, 190_534_818);
        assert!(fatal_error_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn stops_forwarding_after_deserialization_failure() {
        let TestChannels {
            sender,
            mut receiver,
            fatal_error_tx,
            mut fatal_error_rx,
            stopped,
        } = channels();

        assert!(
            forward_block(
                br#"{"block":{"header":{"height":42,"hash":"invalid"}}}"#.to_vec(),
                "aurora",
                sender.clone(),
                fatal_error_tx.clone(),
                Arc::clone(&stopped),
            )
            .await
            .is_err()
        );
        assert!(
            forward_block(BLOCK.to_vec(), "aurora", sender, fatal_error_tx, stopped)
                .await
                .is_err() // Stream was stopped due to previous error
        );

        let err = fatal_error_rx.recv().await.unwrap();
        assert!(err.to_string().contains("at height 42 (invalid)"));
        assert!(receiver.try_recv().is_err());
        assert!(fatal_error_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn reports_unknown_aurora_action_as_fatal_input_error() {
        let TestChannels {
            sender,
            mut receiver,
            fatal_error_tx,
            mut fatal_error_rx,
            stopped,
        } = channels();
        let mut source: serde_json::Value = serde_json::from_slice(BLOCK).unwrap();
        let outcome = source["shards"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .flat_map(|shard| shard["receipt_execution_outcomes"].as_array_mut().unwrap())
            .find(|outcome| outcome["receipt"]["receipt"].get("Action").is_some())
            .unwrap();
        outcome["receipt"]["receiver_id"] = "aurora".into();
        outcome["receipt"]["receipt"]["Action"]["actions"][0] =
            serde_json::json!({"FutureAction": {"unknown_field": true}});

        assert!(
            forward_block(
                serde_json::to_vec(&source).unwrap(),
                "aurora",
                sender,
                fatal_error_tx,
                stopped,
            )
            .await
            .is_err()
        );
        let error = fatal_error_rx.recv().await.unwrap().to_string();
        assert!(error.contains("cannot decode Aurora receipt"));
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn reports_dropped_refiner_receiver() {
        let TestChannels {
            sender,
            receiver,
            fatal_error_tx,
            mut fatal_error_rx,
            stopped,
        } = channels();
        drop(receiver);

        assert!(
            forward_block(BLOCK.to_vec(), "aurora", sender, fatal_error_tx, stopped)
                .await
                .is_err()
        );

        let err = fatal_error_rx.recv().await.unwrap();
        assert_eq!(err.to_string(), "Refiner input receiver was dropped");
    }

    #[tokio::test]
    async fn receiver_dropped_during_shutdown_is_not_an_error() {
        let (fatal_error_tx, mut fatal_error_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
        // The refiner receives the shutdown signal and then drops its input receiver,
        // which fails the pending send.
        shutdown_tx.send(()).unwrap();
        fatal_error_tx
            .send(anyhow::anyhow!("Refiner input receiver was dropped"))
            .unwrap();

        let source_run = std::future::pending::<Result<(), String>>();
        assert!(
            wait_for_input_end(source_run, &mut fatal_error_rx, &mut shutdown_rx)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn fatal_error_is_not_lost_when_source_finishes() {
        let (fatal_error_tx, mut fatal_error_rx) = mpsc::unbounded_channel();
        let (_shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
        fatal_error_tx
            .send(anyhow::anyhow!("invalid block"))
            .unwrap();

        let source_run = std::future::ready(Ok::<(), String>(()));
        let err = wait_for_input_end(source_run, &mut fatal_error_rx, &mut shutdown_rx)
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "invalid block");
    }
}
