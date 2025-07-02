use crate::near_block::{self, NEARBlock};
use near_indexer::StreamerMessage;

pub fn convert(message: StreamerMessage) -> NEARBlock {
    NEARBlock {
        block: message.block.into(),
        shards: message
            .shards
            .into_iter()
            .map(|indexer_shard| {
                let chunk = indexer_shard.chunk.map(|chunk| near_block::ChunkView {
                    author: chunk.author,
                    header: chunk.header.into(),
                    transactions: chunk
                        .transactions
                        .into_iter()
                        .map(|tx| near_block::TransactionWithOutcome {
                            transaction: tx.transaction.into(),
                            outcome: near_block::ExecutionOutcomeWithOptionalReceipt {
                                execution_outcome: tx.outcome.execution_outcome,
                                receipt: tx.outcome.receipt.map(Into::into),
                            },
                        })
                        .collect(),
                    receipts: chunk.receipts.into_iter().map(Into::into).collect(),
                });
                near_block::Shard {
                    shard_id: indexer_shard.shard_id,
                    chunk,
                    receipt_execution_outcomes: indexer_shard
                        .receipt_execution_outcomes
                        .into_iter()
                        .map(|r| near_block::ExecutionOutcomeWithReceipt {
                            execution_outcome: r.execution_outcome,
                            receipt: r.receipt.into(),
                        })
                        .collect(),
                    // Own impl for state changes for backward compatibility, caused by nearcore update from 2.6.3 to 2.7.0-rc.1
                    state_changes: crate::near_block::convert_state_changes(
                        indexer_shard.state_changes,
                    ),
                }
            })
            .collect(),
    }
}
