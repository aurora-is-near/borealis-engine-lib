//! Direct adapter for the typed nearcore indexer stream. The JSON wire DTO is
//! used only for raw stream messages; converting a `StreamerMessage` through
//! JSON would serialize and deserialize the entire block needlessly.

use std::io;

use near_primitives::{types::AccountId, views};

use super::{
    Action, Block, BlockHeader, Chunk, ChunkReceipt, ConversionError, DataReceipt,
    ExecutionOutcome, ExecutionStatus, InnerNearBlock, Receipt, ReceiptExecutionOutcome,
    ReceiptKind, Shard, StateChange, StateChangeCause, Transaction,
};

impl TryFrom<near_indexer::StreamerMessage> for InnerNearBlock {
    type Error = ConversionError;

    fn try_from(source: near_indexer::StreamerMessage) -> Result<Self, Self::Error> {
        let header = source.block.header;
        let shards = source
            .shards
            .into_iter()
            .map(convert_shard)
            .collect::<Result<_, _>>()
            .map_err(|error| ConversionError::new(header.height, header.hash, error))?;

        Ok(Self {
            block: Block {
                author: source.block.author,
                header: BlockHeader {
                    height: header.height,
                    hash: header.hash,
                    prev_hash: header.prev_hash,
                    prev_state_root: header.prev_state_root,
                    timestamp: header.timestamp,
                    timestamp_nanosec: header.timestamp_nanosec,
                    random_value: header.random_value,
                    expected_shards: header.chunk_mask.len(),
                },
            },
            shards,
        })
    }
}

fn convert_shard(source: near_indexer::IndexerShard) -> Result<Shard, io::Error> {
    Ok(Shard {
        chunk: source.chunk.map(|chunk| Chunk {
            transactions: chunk
                .transactions
                .into_iter()
                .map(|tx| Transaction {
                    hash: tx.transaction.hash,
                    receipt_ids: tx.outcome.execution_outcome.outcome.receipt_ids,
                })
                .collect(),
            receipts: chunk
                .receipts
                .into_iter()
                .map(convert_chunk_receipt)
                .collect(),
            local_receipts: chunk
                .local_receipts
                .into_iter()
                .map(convert_chunk_receipt)
                .collect(),
        }),
        receipt_execution_outcomes: source
            .receipt_execution_outcomes
            .into_iter()
            .map(convert_receipt_execution_outcome)
            .collect::<Result<_, _>>()?,
        state_changes: source
            .state_changes
            .into_iter()
            .filter_map(|change| {
                let cause = match change.cause {
                    views::StateChangeCauseView::ReceiptProcessing { receipt_hash } => {
                        StateChangeCause::ReceiptProcessing { receipt_hash }
                    }
                    other => StateChangeCause::Other(format!("{other:?}")),
                };
                convert_state_change(change.value, cause)
            })
            .collect(),
    })
}

fn convert_chunk_receipt(source: views::ReceiptView) -> ChunkReceipt {
    let data = match source.receipt {
        views::ReceiptEnumView::Data { data_id, data, .. } => Some(DataReceipt { data_id, data }),
        _ => None,
    };
    ChunkReceipt {
        receiver_id: source.receiver_id,
        receipt_id: source.receipt_id,
        data,
    }
}

fn convert_receipt_execution_outcome(
    source: near_indexer::IndexerExecutionOutcomeWithReceipt,
) -> Result<ReceiptExecutionOutcome, io::Error> {
    let receipt_size = borsh::object_length(&source.receipt)? as u64;
    let receipt = source.receipt;
    let outcome = source.execution_outcome.outcome;
    Ok(ReceiptExecutionOutcome {
        execution_outcome: ExecutionOutcome {
            logs: outcome.logs,
            receipt_ids: outcome.receipt_ids,
            status: convert_status(outcome.status)?,
        },
        receipt: Receipt {
            predecessor_id: receipt.predecessor_id,
            receiver_id: receipt.receiver_id,
            receipt_id: receipt.receipt_id,
            receipt: convert_receipt_kind(receipt.receipt)?,
        },
        receipt_size: Some(receipt_size),
    })
}

fn convert_status(source: views::ExecutionStatusView) -> Result<ExecutionStatus, io::Error> {
    Ok(match source {
        views::ExecutionStatusView::Unknown => ExecutionStatus::Unknown,
        views::ExecutionStatusView::Failure(error) => {
            ExecutionStatus::Failure(borsh::to_vec(&error)?)
        }
        views::ExecutionStatusView::SuccessValue(value) => ExecutionStatus::SuccessValue(value),
        views::ExecutionStatusView::SuccessReceiptId(id) => ExecutionStatus::SuccessReceiptId(id),
    })
}

fn convert_receipt_kind(source: views::ReceiptEnumView) -> Result<ReceiptKind, io::Error> {
    Ok(match source {
        views::ReceiptEnumView::Action {
            signer_id,
            input_data_ids,
            actions,
            ..
        } => ReceiptKind::Action {
            signer_id,
            input_data_ids,
            actions: actions
                .into_iter()
                .map(convert_action)
                .collect::<Result<_, _>>()?,
        },
        views::ReceiptEnumView::Data { data_id, .. } => ReceiptKind::Data { data_id },
        views::ReceiptEnumView::GlobalContractDistribution { .. } => {
            ReceiptKind::GlobalContractDistribution
        }
    })
}

fn convert_action(source: views::ActionView) -> Result<Action, io::Error> {
    Ok(match source {
        views::ActionView::FunctionCall {
            method_name,
            args,
            deposit,
            ..
        } => Action::FunctionCall {
            method_name,
            args: args.into(),
            deposit: deposit.as_yoctonear(),
        },
        other => Action::Other {
            borsh_bytes: borsh::to_vec(&other)?,
        },
    })
}

fn convert_state_change(
    value: views::StateChangeValueView,
    cause: StateChangeCause,
) -> Option<StateChange> {
    let (account_id, key, value): (AccountId, Vec<u8>, Option<Vec<u8>>) = match value {
        views::StateChangeValueView::DataUpdate {
            account_id,
            key,
            value,
        } => (account_id, key.into(), Some(value.into())),
        views::StateChangeValueView::DataDeletion { account_id, key } => {
            (account_id, key.into(), None)
        }
        _ => return None,
    };
    Some(StateChange {
        account_id,
        key,
        value,
        cause,
    })
}
