use aurora_refiner_types::conversion::Converter;
use aurora_refiner_types::near_block::{ChunkHeaderView, ChunkView, NEARBlock, Shard};
use near_lake_framework::near_indexer_primitives::StreamerMessage;

pub fn convert(block: StreamerMessage) -> NEARBlock {
    NEARBlock {
        block: block.block.into(),
        shards: block
            .shards
            .into_iter()
            .map(|shard| Shard {
                shard_id: shard.shard_id.convert(),
                chunk: shard.chunk.map(|chunk| ChunkView {
                    author: chunk.author.convert(),
                    header: ChunkHeaderView {
                        chunk_hash: chunk.header.chunk_hash.convert(),
                        prev_block_hash: chunk.header.prev_block_hash.convert(),
                        outcome_root: chunk.header.outcome_root.convert(),
                        prev_state_root: chunk.header.prev_state_root.convert(),
                        encoded_merkle_root: chunk.header.encoded_merkle_root.convert(),
                        encoded_length: chunk.header.encoded_length,
                        height_created: chunk.header.height_created,
                        height_included: chunk.header.height_included,
                        shard_id: chunk.header.shard_id.convert(),
                        gas_used: chunk.header.gas_used,
                        gas_limit: chunk.header.gas_limit,
                        validator_reward: chunk.header.validator_reward,
                        balance_burnt: chunk.header.balance_burnt,
                        outgoing_receipts_root: chunk.header.outgoing_receipts_root.convert(),
                        tx_root: chunk.header.tx_root.convert(),
                        validator_proposals: chunk
                            .header
                            .validator_proposals
                            .iter()
                            .map(Converter::convert)
                            .collect(),
                        signature: chunk.header.signature.convert(),
                    },
                    transactions: ch_json(chunk.transactions),
                    receipts: ch_json(chunk.receipts),
                }),
                receipt_execution_outcomes: ch_json(shard.receipt_execution_outcomes),
                state_changes: ch_json(shard.state_changes),
            })
            .collect(),
    }
}

// /// Convert between types that have the same json representation
// pub fn ch_json<U: Serialize, V: DeserializeOwned>(input: U) -> V {
//     let value = serde_json::to_value(input).unwrap();
//     serde_json::from_value(value).unwrap()
// }
