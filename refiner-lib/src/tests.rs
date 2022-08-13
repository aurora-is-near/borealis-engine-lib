use std::{collections::HashSet, path::PathBuf};

use crate::near_stream::NearStream;
use aurora_refiner_types::{aurora_block::AuroraBlock, near_block::NEARBlock};
use aurora_standalone_engine::EngineContext;
use engine_standalone_storage::Storage;

fn load_near_block(block_height: u64) -> NEARBlock {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(format!("blocks/block-{}.json", block_height));
    let file = std::fs::File::open(path).unwrap();

    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}

fn aurora_block_from_near_block(block_height: u64) -> AuroraBlock {
    const CHAIN_ID: u64 = 1313161554;
    const ENGINE_ACCOUNT_ID: &str = "aurora";
    const STORAGE_PATH: &str = "test-storage";

    let block = load_near_block(block_height);

    {
        let mut storage = Storage::open(STORAGE_PATH).unwrap();
        storage
            .set_engine_account_id(&ENGINE_ACCOUNT_ID.parse().unwrap())
            .unwrap();
    }

    let ctx =
        EngineContext::new(STORAGE_PATH, ENGINE_ACCOUNT_ID.parse().unwrap(), CHAIN_ID).unwrap();

    let mut stream = NearStream::new(CHAIN_ID, Some(block_height - 1), ctx);

    let blocks = stream.next_block(block);
    assert_eq!(blocks.len(), 1);

    blocks.into_iter().next().unwrap()
}

#[test]
fn test_block_aurora_genesis() {
    let block = aurora_block_from_near_block(34834053);
    assert_eq!(block.transactions.len(), 3);
}

/// Process NEAR block at height 51188690, and check that there are only 3 transactions with different hashes.
#[test]
fn test_block_51188690() {
    let block = aurora_block_from_near_block(51188690);
    assert_eq!(block.transactions.len(), 7);
    let mut set = HashSet::new();
    block.transactions.iter().for_each(|tx| {
        set.insert(tx.hash);
    });
    assert_eq!(set.len(), 7);
}

#[test]
fn test_block_51188689() {
    let block = aurora_block_from_near_block(51188689);
    assert_eq!(block.transactions.len(), 1);
}
