use crate::near_stream::NearStream;
use aurora_refiner_types::near_block::NEARBlock;
use aurora_standalone_engine::EngineContext;

fn load_block(block_height: u64) -> NEARBlock {
    let file = std::fs::File::open(format!("blocks/block-{}.json", block_height)).unwrap();
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}

#[test]
fn test_block_aurora_genesis() {
    let block = load_block(34834053);
    let ctx = EngineContext::new("test-storage", "aurora".parse().unwrap(), 1313161554).unwrap();
    let mut stream = NearStream::new(1313161554, Some(34834053 - 1), ctx);
    let blocks = stream.next_block(block);
    assert_eq!(blocks.len(), 1);
    let block = blocks.into_iter().next().unwrap();
    assert_eq!(block.transactions.len(), 3);
}
