use std::path::{Path, PathBuf};

use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::aurora_block::AuroraBlock;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;

const STORE_INFO_FILE: &str = ".REFINER_LAST_BLOCK";

#[derive(Clone, Debug, Deserialize)]
pub struct OutputStoreConfig {
    /// Path to the folder where all blocks will be stored
    pub path: String,
    /// Number of files (blocks) to store on each folder.
    pub batch_size: u64,
}

pub async fn store(config: &OutputStoreConfig, block: &AuroraBlock) {
    tracing::trace!("Storing block {}", block.height);
    let folder_path = PathBuf::from(&config.path);

    if !folder_path.exists() {
        std::fs::create_dir_all(&folder_path).unwrap();
    }

    let mut tmp_path = folder_path.clone();
    tmp_path.push(".PARTIAL");

    let file = tokio::fs::File::create(&tmp_path).await.unwrap();

    {
        let mut writer = tokio::io::BufWriter::new(file);
        let data = serde_json::to_string(block).unwrap();
        writer.write_all(data.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    }

    let mut target_path = folder_path;
    target_path.push(format!(
        "{}",
        block.height - block.height % config.batch_size
    ));

    if !target_path.exists() {
        tokio::fs::create_dir_all(&target_path).await.unwrap();
    }

    target_path.push(format!("{}.json", block.height));
    tracing::trace!(
        "Moving {} to {}.",
        tmp_path.display(),
        target_path.display()
    );
    tokio::fs::rename(tmp_path, target_path).await.unwrap();

    save_last_block_height(&config.path, block.height).await;
}

/// Spawns a task that stores Aurora blocks to the output storage.
/// The `shutdown_rx` is used to signal the task to stop.
/// Returns a channel to send Aurora blocks to the task and a handle to the task.
pub fn get_output_stream(
    mut total_blocks: Option<u64>,
    config: OutputStoreConfig,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> (
    tokio::sync::mpsc::Sender<BlockWithMetadata<AuroraBlock, ()>>,
    tokio::task::JoinHandle<()>,
) {
    tracing::info!("get_output_stream: starting output stream, total_blocks: {total_blocks:?}...");
    let (aurora_blocks_tx, mut aurora_blocks_rx) =
        tokio::sync::mpsc::channel::<BlockWithMetadata<AuroraBlock, ()>>(1000);

    let task_handle = tokio::spawn(async move {
        let config = config.clone();

        loop {
            tokio::select! {
                // Handle incoming blocks
                Some(block) = aurora_blocks_rx.recv() => {
                    store(&config, &block.block).await;
                    if let Some(total_blocks) = total_blocks.as_mut() {
                        *total_blocks -= 1;
                        if *total_blocks == 0 {
                            break;
                        }
                    }
                }
                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    // Explicitly close the channel, so the tx side should stop sending blocks
                    aurora_blocks_rx.close();
                    info!("get_output_stream: Received shutdown signal, Aurora blocks storage stopped");
                    break;
                }
            }
        }
    });

    tracing::info!("get_output_stream: output stream finished");

    (aurora_blocks_tx, task_handle)
}

pub async fn load_last_block_height<P: AsRef<Path> + Send>(storage_path: P) -> Option<u64> {
    let path = storage_path.as_ref();
    if !path.exists() {
        tokio::fs::create_dir_all(path).await.unwrap();
    }
    let store_file = path.join(STORE_INFO_FILE);

    if store_file.exists() {
        let mut file = tokio::fs::File::open(&store_file).await.unwrap();
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).await.unwrap();
        Some(buffer.trim().parse().unwrap())
    } else {
        None
    }
}

async fn save_last_block_height<P: AsRef<Path> + Send>(storage_path: P, block_height: u64) {
    let path = storage_path.as_ref();
    if !path.exists() {
        tokio::fs::create_dir_all(path).await.unwrap();
    }
    let file_path = path.join(STORE_INFO_FILE);

    // Write the data to a height-specific file to avoid clearing the main file
    let temp_path = path.join(format!(".{block_height}"));
    let temp_file = tokio::fs::File::create(&temp_path).await.unwrap();

    {
        let mut writer = tokio::io::BufWriter::new(temp_file);
        let data = block_height.to_string();
        writer.write_all(data.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    }

    // Move the height-specific file to the main file, thus atomically updating it.
    tokio::fs::rename(temp_path, file_path).await.unwrap();

    tracing::trace!("Last block height {} saved.", block_height);
}

#[cfg(test)]
mod tests {
    use super::{load_last_block_height, save_last_block_height};

    #[tokio::test]
    async fn test_save_last_block_height() {
        const HEIGHT: u64 = 11111;
        let tmp_dir = tempfile::tempdir().unwrap();
        save_last_block_height(tmp_dir.path(), HEIGHT).await;
        let block_height = load_last_block_height(tmp_dir.path()).await;

        assert_eq!(block_height, Some(HEIGHT))
    }
}
