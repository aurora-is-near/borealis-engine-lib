use aurora_refiner_types::near_block::{ChunkHeaderView, ChunkView, NEARBlock, Shard};
use near_lake_framework::near_indexer_primitives::StreamerMessage;
use serde::{Serialize, de::DeserializeOwned};

pub fn convert(block: StreamerMessage) -> NEARBlock {
    NEARBlock {
        block: ch_json(block.block),
        shards: block
            .shards
            .into_iter()
            .map(|shard| Shard {
                shard_id: ch_json(shard.shard_id),
                chunk: shard.chunk.map(|chunk| ChunkView {
                    author: ch_json(chunk.author),
                    header: ChunkHeaderView {
                        chunk_hash: ch_json(chunk.header.chunk_hash),
                        prev_block_hash: ch_json(chunk.header.prev_block_hash),
                        outcome_root: ch_json(chunk.header.outcome_root),
                        prev_state_root: ch_json(chunk.header.prev_state_root),
                        encoded_merkle_root: ch_json(chunk.header.encoded_merkle_root),
                        encoded_length: ch_json(chunk.header.encoded_length),
                        height_created: ch_json(chunk.header.height_created),
                        height_included: ch_json(chunk.header.height_included),
                        shard_id: ch_json(chunk.header.shard_id),
                        gas_used: ch_json(chunk.header.gas_used),
                        gas_limit: ch_json(chunk.header.gas_limit),
                        validator_reward: chunk.header.validator_reward,
                        balance_burnt: chunk.header.balance_burnt,
                        outgoing_receipts_root: ch_json(chunk.header.outgoing_receipts_root),
                        tx_root: ch_json(chunk.header.tx_root),
                        validator_proposals: ch_json(chunk.header.validator_proposals),
                        signature: ch_json(chunk.header.signature),
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

/// Convert between types that have the same json representation
pub fn ch_json<U: Serialize, V: DeserializeOwned>(input: U) -> V {
    let value = serde_json::to_value(input).unwrap();
    serde_json::from_value(value).unwrap()
}

pub mod data_lake {
    use aurora_refiner_types::near_block::{NEARBlock, convert_execution_outcome_with_id_view};
    use near_lake_framework::near_indexer_primitives::StreamerMessage;

    pub fn convert(message: StreamerMessage) -> NEARBlock {
        NEARBlock {
            block: message.block.into(),
            shards: message
                .shards
                .into_iter()
                .map(|indexer_shard| {
                    let chunk = indexer_shard.chunk.map(|chunk| {
                        aurora_refiner_types::near_block::ChunkView {
                            author: chunk.author,
                            header: chunk.header.into(),
                            transactions: chunk
                                .transactions
                                .into_iter()
                                .map(|tx| {
                                    aurora_refiner_types::near_block::TransactionWithOutcome {
                                        transaction: tx.transaction.into(),
                                        outcome: tx.outcome.into(),
                                    }
                                })
                                .collect(),
                            receipts: chunk.receipts.into_iter().map(Into::into).collect(),
                        }
                    });
                    aurora_refiner_types::near_block::Shard {
                        shard_id: aurora_refiner_types::near_block::convert_shard_id(
                            indexer_shard.shard_id,
                        ),
                        chunk,
                        receipt_execution_outcomes: indexer_shard
                            .receipt_execution_outcomes
                            .into_iter()
                            .map(|r| {
                                aurora_refiner_types::near_block::ExecutionOutcomeWithReceipt {
                                    execution_outcome: convert_execution_outcome_with_id_view(
                                        r.execution_outcome,
                                    ),
                                    receipt: r.receipt.into(),
                                }
                            })
                            .collect(),
                        state_changes: aurora_refiner_types::near_block::convert_state_changes_view(
                            indexer_shard.state_changes,
                        ),
                    }
                })
                .collect(),
        }
    }
}

pub mod nearcore {
    use aurora_refiner_types::near_block::NEARBlock;
    use near_indexer::StreamerMessage;

    pub fn convert(message: StreamerMessage) -> NEARBlock {
        NEARBlock {
            block: message.block.into(),
            shards: message.shards.into_iter()
                .map(|indexer_shard| {
                    let chunk = indexer_shard.chunk.map(|chunk| {
                        aurora_refiner_types::near_block::ChunkView {
                            author: chunk.author,
                            header: chunk.header.into(),
                            transactions: chunk.transactions
                                .into_iter()
                                .map(|tx| {
                                    aurora_refiner_types::near_block::TransactionWithOutcome {
                                        transaction: tx.transaction.into(),
                                        outcome: aurora_refiner_types::near_block::ExecutionOutcomeWithOptionalReceipt {
                                            execution_outcome: tx.outcome.execution_outcome,
                                            receipt: tx.outcome.receipt.map(Into::into),
                                        },
                                    }
                                })
                                .collect(),
                            receipts: chunk.receipts.into_iter().map(Into::into).collect(),
                        }
                    });
                    aurora_refiner_types::near_block::Shard {
                        shard_id: indexer_shard.shard_id,
                        chunk,
                        receipt_execution_outcomes: indexer_shard.receipt_execution_outcomes
                            .into_iter()
                            .map(|r| {
                                aurora_refiner_types::near_block::ExecutionOutcomeWithReceipt {
                                    execution_outcome: r.execution_outcome,
                                    receipt: r.receipt.into(),
                                }
                            })
                            .collect(),
                        state_changes: indexer_shard.state_changes,
                    }
                })
                .collect()
        }
    }
}
