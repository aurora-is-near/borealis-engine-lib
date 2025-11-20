use crate::{
    Converter,
    near_block::{self, NEARBlock},
};
use near_lake_framework::near_indexer_primitives::StreamerMessage;

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
                            outcome: tx.outcome.into(),
                        })
                        .collect(),
                    receipts: chunk.receipts.into_iter().map(Into::into).collect(),
                    local_receipts: chunk.local_receipts.into_iter().map(Into::into).collect(),
                });
                near_block::Shard {
                    shard_id: indexer_shard.shard_id.convert(),
                    chunk,
                    receipt_execution_outcomes: indexer_shard
                        .receipt_execution_outcomes
                        .into_iter()
                        .map(|r| near_block::ExecutionOutcomeWithReceipt {
                            execution_outcome: r.execution_outcome.convert(),
                            receipt: r.receipt.into(),
                        })
                        .collect(),
                    state_changes: indexer_shard.state_changes.convert(),
                }
            })
            .collect(),
    }
}
