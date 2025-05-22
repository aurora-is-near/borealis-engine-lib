use criterion::{Criterion, criterion_group, criterion_main};

/// Benchmark the performance of both `data_lake` and `nearcore` convert functions.
///
/// The functions converts:
/// 1. `near_indexer::StreamerMessage` into `aurora_refiner_types::near_block::NEARBlock`;
/// 2. `near_lake_framework::near_indexer_primitives::StreamerMessage` into `aurora_refiner_types::near_block::NEARBlock`.
pub fn convert(c: &mut Criterion) {
    let streamer_message_nearcore: near_indexer::StreamerMessage = serde_json::from_str(
        include_str!("../tests/res/streamer_message_190534818_branch_remove_custom_indexer.json"),
    )
    .unwrap();

    let streamer_message_data_lake: near_lake_framework::near_indexer_primitives::StreamerMessage =
        serde_json::from_str(include_str!(
            "../tests/res/streamer_message_190534818_branch_remove_custom_indexer.json"
        ))
        .unwrap();

    let mut group = c.benchmark_group("convert_group");
    group.sample_size(500);

    group.bench_function("aurora_refiner_types::conversion::nearcore::convert", |b| {
        b.iter(|| {
            aurora_refiner_types::conversion::nearcore::convert(std::hint::black_box(
                streamer_message_nearcore.clone(),
            ))
        })
    });

    group.bench_function(
        "aurora_refiner_types::conversion::data_lake::convert",
        |b| {
            b.iter(|| {
                aurora_refiner_types::conversion::data_lake::convert(std::hint::black_box(
                    streamer_message_data_lake.clone(),
                ))
            })
        },
    );

    group.finish();
}

criterion_group!(benches, convert);
criterion_main!(benches);
