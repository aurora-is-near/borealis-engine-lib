use aurora_refiner_types::inner_block::InnerNearBlock;
use criterion::{Criterion, criterion_group, criterion_main};

const BLOCK_JSON_STR: &str =
    include_str!("../tests/res/streamer_message_190534818_branch_remove_custom_indexer.json");
const BLOCK_JSON_BYTES: &[u8] =
    include_bytes!("../tests/res/streamer_message_190534818_branch_remove_custom_indexer.json");

/// Benchmark for measuring `InnerNearBlock` conversion from `StreamerMessage` and block JSON.
pub fn convert(c: &mut Criterion) {
    let streamer_message: near_indexer::StreamerMessage =
        serde_json::from_str(BLOCK_JSON_STR).unwrap();
    let block_bytes = BLOCK_JSON_BYTES;

    let mut group = c.benchmark_group("convert_group");
    group.sample_size(500);

    group.bench_function("InnerNearBlock::try_from::StreamerMessage", |b| {
        b.iter(|| InnerNearBlock::try_from(std::hint::black_box(streamer_message.clone())).unwrap())
    });
    group.bench_function("StreamerMessage JSON roundtrip", |b| {
        b.iter(|| {
            let message = std::hint::black_box(streamer_message.clone());
            let bytes = serde_json::to_vec(&message).unwrap();
            InnerNearBlock::from_bytes(std::hint::black_box(&bytes)).unwrap()
        })
    });
    group.bench_function("InnerNearBlock::from_bytes", |b| {
        b.iter(|| {
            let block = InnerNearBlock::from_bytes(block_bytes).unwrap();
            std::hint::black_box(block)
        })
    });

    group.finish();
}

criterion_group!(benches, convert);
criterion_main!(benches);
