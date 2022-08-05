use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::aurora_block::AuroraBlock;
use serde::Deserialize;

const STORE_INFO_FILE: &str = ".REFINER_LAST_BLOCK";

#[derive(Clone, Debug, Deserialize)]
pub struct OutputStoreConfig {
    /// Path to the folder where all blocks will be stored
    pub path: String,
    /// Number of files (blocks) to store on each folder.
    pub batch_size: u64,
}

pub fn store(config: &OutputStoreConfig, block: &AuroraBlock) {
    let folder_path = std::path::PathBuf::from(&config.path);

    if !folder_path.exists() {
        std::fs::create_dir_all(&folder_path).unwrap();
    }

    let mut tmp_path = folder_path.clone();
    tmp_path.push(".PARTIAL");
    let file = std::fs::File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&tmp_path)
        .unwrap();

    serde_json::to_writer(&file, block).unwrap();

    let mut target_path = folder_path;
    target_path.push(format!(
        "{}",
        block.height - block.height % config.batch_size
    ));

    if !target_path.exists() {
        std::fs::create_dir_all(&target_path).unwrap();
    }

    target_path.push(format!("{}.json", block.height));
    std::fs::rename(tmp_path, target_path).unwrap();

    save_last_block_height(&config.path, block.height);
}

pub fn get_output_stream(
    config: OutputStoreConfig,
) -> tokio::sync::mpsc::Sender<BlockWithMetadata<AuroraBlock, ()>> {
    let (sender, mut receiver) =
        tokio::sync::mpsc::channel::<BlockWithMetadata<AuroraBlock, ()>>(1000);

    tokio::spawn(async move {
        let config = config.clone();
        while let Some(block) = receiver.recv().await {
            store(&config, &block.block);
        }
    });

    sender
}

pub fn load_last_block_height<P: AsRef<std::path::Path>>(storage_path: P) -> Option<u64> {
    let path = storage_path.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(path).unwrap();
    }
    let store_file = path.join(STORE_INFO_FILE);

    store_file.exists().then(|| {
        let file = std::fs::File::open(&store_file).unwrap();
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).unwrap()
    })
}

fn save_last_block_height<P: AsRef<std::path::Path>>(storage_path: P, block_height: u64) {
    let path = storage_path.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(path).unwrap();
    }
    let path = path.join(STORE_INFO_FILE);

    let file = std::fs::File::create(path).unwrap();
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, &block_height).unwrap();
}
