use aurora_refiner_lib::BlockWithMetadata;
use aurora_refiner_types::near_block::NEARBlock;
use near_lake_framework::LakeConfigBuilder;

use crate::{config::DataLakeConfig, conversion::convert};

pub fn get_near_data_lake_stream(
    block_height: u64,
    config: &DataLakeConfig,
) -> tokio::sync::mpsc::Receiver<BlockWithMetadata<NEARBlock, ()>> {
    let mut opts = LakeConfigBuilder::default();
    opts = match config.network {
        crate::config::Network::Mainnet => opts.mainnet(),
        crate::config::Network::Testnet => opts.testnet(),
    };
    let opts = opts
        .start_block_height(block_height)
        .build()
        .expect("Failed to build LakeConfig");

    let (sender, receiver) = tokio::sync::mpsc::channel(1000);

    tokio::spawn(async move {
        // instantiate the NEAR Lake Framework Stream
        let mut stream = near_lake_framework::streamer(opts);
        while let Some(block) = stream.recv().await {
            sender
                .send(BlockWithMetadata::new(convert(block), ()))
                .await
                .unwrap();
        }
    });

    receiver
}
