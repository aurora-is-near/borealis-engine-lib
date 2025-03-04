use aurora_engine::parameters;
use aurora_engine_modexp::ModExpAlgorithm;
use aurora_engine_sdk::env;
use aurora_engine_types::borsh::BorshDeserialize;
use aurora_engine_types::{H256, account_id::AccountId};
use aurora_refiner_types::near_primitives::{
    self,
    hash::CryptoHash,
    views::{ActionView, StateChangeValueView},
};
use engine_standalone_storage::sync::types::TransactionKind;
use engine_standalone_storage::{
    BlockMetadata, Diff, Storage,
    sync::{
        self, ConsumeMessageOutcome, TransactionExecutionResult, TransactionIncludedOutcome,
        types::{self, Message},
    },
};
use lru::LruCache;
use std::{cell::RefCell, collections::HashMap};
use tracing::{debug, warn};

use crate::batch_tx_processing::BatchIO;

#[allow(clippy::cognitive_complexity, clippy::option_if_let_else)]
pub fn consume_near_block<M: ModExpAlgorithm>(
    storage: &mut Storage,
    message: &aurora_refiner_types::near_block::NEARBlock,
    data_id_mapping: &mut LruCache<CryptoHash, Option<Vec<u8>>>,
    engine_account_id: &AccountId,
    chain_id: [u8; 32],
    mut outcomes: Option<&mut HashMap<H256, TransactionIncludedOutcome>>,
) -> Result<(), engine_standalone_storage::Error> {
    let block_hash =
        add_block_data_from_near_block::<M>(storage, message, chain_id, engine_account_id)?;
    let near_block_hash = &message.block.header.hash;

    // Capture data receipts (for using in promises)
    message
        .shards
        .iter()
        .filter_map(|shard| shard.chunk.as_ref())
        .flat_map(|chunk| chunk.receipts.as_slice())
        .for_each(|r| {
            if r.receiver_id.as_str() == engine_account_id.as_ref() {
                if let near_primitives::views::ReceiptEnumView::Data { data_id, data, .. } =
                    &r.receipt
                {
                    data_id_mapping.put(*data_id, data.clone());
                }
            }
        });

    // Get expected state changes based on data in the streamer message
    let aurora_state_changes = message
        .shards
        .iter()
        .flat_map(|s| s.state_changes.iter())
        .filter_map(|change| match &change.value {
            StateChangeValueView::DataUpdate {
                account_id,
                key,
                value,
            } => Some((account_id, key, Some(value), &change.cause)),
            StateChangeValueView::DataDeletion { account_id, key } => {
                Some((account_id, key, None, &change.cause))
            }
            _ => None,
        })
        .filter(|(account_id, _, _, _)| account_id.as_str() == engine_account_id.as_ref());

    let mut expected_diffs: HashMap<H256, Diff> = HashMap::new();
    for (_, key, expected_value, cause) in aurora_state_changes {
        let receipt_id: H256 = match cause {
            near_primitives::views::StateChangeCauseView::ReceiptProcessing { receipt_hash } => {
                receipt_hash.0.into()
            }
            other => panic!("Unexpected state change cause {:?}", other),
        };

        let diff = expected_diffs.entry(receipt_id).or_default();
        match expected_value {
            Some(value) => diff.modify(key.to_vec(), value.to_vec()),
            None => diff.delete(key.to_vec()),
        }
    }

    let mut position_counter = 0;
    let transaction_messages = message
        .shards
        .iter()
        .flat_map(|shard| shard.receipt_execution_outcomes.iter())
        .filter_map(|outcome| {
            if outcome.receipt.receiver_id.as_str() != engine_account_id.as_ref() {
                return None;
            }

            // Ignore failed transactions since they do not impact the engine state
            let execution_result_bytes = match &outcome.execution_outcome.outcome.status {
                near_primitives::views::ExecutionStatusView::Unknown => return None,
                near_primitives::views::ExecutionStatusView::Failure(_) => return None,
                near_primitives::views::ExecutionStatusView::SuccessValue(bytes) => Some(bytes),
                near_primitives::views::ExecutionStatusView::SuccessReceiptId(_) => None,
            };

            let (signer, maybe_tx, promise_data) = match &outcome.receipt.receipt {
                near_primitives::views::ReceiptEnumView::Action {
                    signer_id,
                    actions,
                    input_data_ids,
                    ..
                } => {
                    let input_data: Vec<_> = input_data_ids
                        .iter()
                        .map(|id| data_id_mapping.pop(id).flatten())
                        .collect();
                    let maybe_tx = parse_actions(actions, &input_data);

                    (signer_id, maybe_tx, input_data)
                }
                near_primitives::views::ReceiptEnumView::Data { .. } => return None,
            };

            let signer = signer.as_str().parse().ok()?;
            let caller = outcome.receipt.predecessor_id.as_str().parse().ok()?;
            let near_receipt_id = outcome.receipt.receipt_id.0.into();
            let maybe_batch_actions = match maybe_tx {
                Some(tn) => tn,
                None => {
                    if expected_diffs.contains_key(&near_receipt_id) {
                        warn!(
                            "Receipt {:?} not parsed as transaction, but has state changes",
                            near_receipt_id,
                        );
                        ParsedActions::Single(SingleParsedAction::default())
                    } else {
                        return None;
                    }
                }
            };

            let transaction_messages = match maybe_batch_actions {
                ParsedActions::Single(parsed_action) => {
                    let action_hash =
                        compute_action_hash(&outcome.receipt.receipt_id, near_block_hash, 0);
                    let transaction_message = types::TransactionMessage {
                        block_hash,
                        near_receipt_id,
                        position: position_counter,
                        succeeded: true, // we drop failed transactions above
                        signer,
                        caller,
                        attached_near: parsed_action.deposit,
                        transaction: *parsed_action.transaction_kind,
                        promise_data,
                        raw_input: parsed_action.raw_input,
                        action_hash,
                    };
                    position_counter += 1;

                    TransactionBatch::Single(transaction_message)
                }

                ParsedActions::Batch(txs) => {
                    let mut non_last_actions: Vec<_> = txs
                        .into_iter()
                        .map(|(index, parsed_action)| {
                            let action_index = match index {
                                BatchIndex::Index(i) => i.into(),
                                BatchIndex::Last(i) => i.into(),
                            };
                            let action_hash = compute_action_hash(
                                &outcome.receipt.receipt_id,
                                near_block_hash,
                                action_index,
                            );
                            let virtual_receipt_id = match index {
                                BatchIndex::Index(i) => {
                                    let mut bytes = [0u8; 36];
                                    bytes[0..32].copy_from_slice(near_receipt_id.as_bytes());
                                    bytes[32..36].copy_from_slice(&i.to_be_bytes());
                                    aurora_refiner_types::utils::keccak256(&bytes)
                                }
                                BatchIndex::Last(_) => near_receipt_id,
                            };
                            let transaction_message = types::TransactionMessage {
                                block_hash,
                                near_receipt_id: virtual_receipt_id,
                                position: position_counter,
                                succeeded: true, // we drop failed transactions above
                                signer: signer.clone(),
                                caller: caller.clone(),
                                attached_near: parsed_action.deposit,
                                transaction: *parsed_action.transaction_kind,
                                promise_data: promise_data.clone(),
                                raw_input: parsed_action.raw_input,
                                action_hash,
                            };
                            position_counter += 1;

                            transaction_message
                        })
                        .collect();

                    let has_last_action = non_last_actions
                        .last()
                        .map(|t| t.near_receipt_id == near_receipt_id)
                        .unwrap_or(false);
                    let last_action = if has_last_action {
                        non_last_actions.pop()
                    } else {
                        None
                    };

                    TransactionBatch::Batch {
                        near_receipt_id,
                        non_last_actions,
                        last_action,
                    }
                }
            };

            Some((transaction_messages, execution_result_bytes))
        });

    for (t, result_bytes) in transaction_messages {
        let receipt_id = t.near_receipt_id();
        debug!("Processing receipt {:?}", receipt_id);
        let tx_outcome = t.process::<M>(storage)?;
        let computed_result = match &tx_outcome {
            TransactionBatchOutcome::Single(tx_outcome) => tx_outcome
                .maybe_result
                .as_ref()
                .map(|x| x.as_ref())
                .ok()
                .flatten(),
            TransactionBatchOutcome::Batch { last_outcome, .. } => {
                last_outcome.as_ref().and_then(|tx_outcome| {
                    tx_outcome
                        .maybe_result
                        .as_ref()
                        .map(|x| x.as_ref())
                        .ok()
                        .flatten()
                })
            }
        };
        // Validate result (note: only the result of the last action in a batch is returned in NEAR)
        if let Some(TransactionExecutionResult::Submit(submit_result)) = computed_result {
            match result_bytes.as_ref() {
                Some(result_bytes) => {
                    match parameters::SubmitResult::try_from_slice(result_bytes) {
                        Ok(expected_result) => {
                            if submit_result.is_err()
                                || submit_result.as_ref().unwrap() != &expected_result
                            {
                                warn!(
                                    "Incorrect result in processing receipt_id={receipt_id:?} computed differed from expected",
                                );
                            }
                        }
                        Err(_) => {
                            warn!("Unable to deserialize receipt_id={receipt_id:?} as SubmitResult",)
                        }
                    }
                }
                None => warn!(
                    "Expected receipt_id={receipt_id:?} to have a return result, but there was none",
                ),
            }
        }
        // Validate against expected diff
        match expected_diffs.get(&receipt_id) {
            None => {
                if !tx_outcome.diff().is_empty() {
                    warn!(
                        "Receipt {receipt_id:?} not expected to have changes, but standalone computed a non-empty diff",
                    );
                }
            }
            Some(expected_diff) => {
                if expected_diff == tx_outcome.diff() {
                    // Diff was correct, so commit it to the storage
                    tx_outcome.commit(storage)?;
                } else {
                    // Diff was incorrect, so log a warning and commit
                    // the one from the Near block instead
                    warn!("Receipt {receipt_id:?} diff mismatch with computed diff");
                    tx_outcome.update_diff(storage, expected_diff)?;
                }
            }
        }
        // Return the computed outcomes
        if let Some(output_outcomes) = outcomes.as_mut() {
            match tx_outcome {
                TransactionBatchOutcome::Single(tx_outcome) => {
                    output_outcomes.insert(tx_outcome.hash, *tx_outcome);
                }
                TransactionBatchOutcome::Batch {
                    non_last_outcomes,
                    last_outcome,
                    ..
                } => {
                    for tx_outcome in non_last_outcomes {
                        output_outcomes.insert(tx_outcome.hash, tx_outcome);
                    }
                    if let Some(tx_outcome) = last_outcome {
                        output_outcomes.insert(tx_outcome.hash, *tx_outcome);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Based on nearcore implementation:
/// <https://github.com/near/nearcore/blob/00ca2f3f73e2a547ba881f76ecc59450dbbef6e2/core/primitives/src/utils.rs#L261>
fn compute_action_hash(
    receipt_id: &CryptoHash,
    near_block_hash: &CryptoHash,
    action_index: u64,
) -> H256 {
    const BYTES_LEN: usize = 32 + 32 + 8;
    let mut bytes: Vec<u8> = Vec::with_capacity(BYTES_LEN);
    bytes.extend_from_slice(receipt_id.as_ref());
    bytes.extend_from_slice(near_block_hash.as_ref());
    bytes.extend_from_slice(&(u64::MAX - action_index).to_le_bytes());
    let hash = near_primitives::hash::hash(&bytes);
    H256(hash.0)
}

fn add_block_data_from_near_block<M: ModExpAlgorithm>(
    storage: &mut Storage,
    message: &aurora_refiner_types::near_block::NEARBlock,
    chain_id: [u8; 32],
    account_id: &AccountId,
) -> Result<H256, engine_standalone_storage::Error> {
    let block_height = message.block.header.height;
    let block_hash =
        aurora_engine::engine::compute_block_hash(chain_id, block_height, account_id.as_bytes());
    let block_message = types::BlockMessage {
        height: block_height,
        hash: block_hash,
        metadata: BlockMetadata {
            timestamp: env::Timestamp::new(message.block.header.timestamp_nanosec),
            random_seed: message.block.header.random_value.0.into(),
        },
    };

    debug!("Consuming block {}", block_message.height);
    sync::consume_message::<M>(storage, Message::Block(block_message))?;

    Ok(block_hash)
}

/// We treat the last element of a batch differently from the rest because its outcome is the outcome
/// of the whole receipt. This enum tags the elements of a batch for downstream processing.
enum BatchIndex {
    Index(u32),
    Last(u32),
}

/// Most NEAR receipts are not batches, so we want to optimize for the case where there is just one
/// action (not allocate a vec every time). This enum enables that optimization.
enum ParsedActions {
    Single(SingleParsedAction),
    Batch(Vec<(BatchIndex, SingleParsedAction)>),
}

struct SingleParsedAction {
    pub transaction_kind: Box<TransactionKind>,
    pub raw_input: Vec<u8>,
    pub deposit: u128,
}

impl Default for SingleParsedAction {
    fn default() -> Self {
        Self {
            transaction_kind: Box::new(TransactionKind::Unknown),
            raw_input: Vec::new(),
            deposit: 0,
        }
    }
}

enum TransactionBatch {
    Single(types::TransactionMessage),
    Batch {
        near_receipt_id: H256,
        non_last_actions: Vec<types::TransactionMessage>,
        last_action: Option<types::TransactionMessage>,
    },
}

impl TransactionBatch {
    const fn near_receipt_id(&self) -> H256 {
        match self {
            Self::Single(tx) => tx.near_receipt_id,
            Self::Batch {
                near_receipt_id, ..
            } => *near_receipt_id,
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn process<M: ModExpAlgorithm>(
        self,
        storage: &mut Storage,
    ) -> Result<TransactionBatchOutcome, engine_standalone_storage::Error> {
        match self {
            Self::Single(tx) => {
                match sync::consume_message::<M>(storage, Message::Transaction(Box::new(tx)))? {
                    ConsumeMessageOutcome::TransactionIncluded(tx_outcome) => {
                        debug!("COMPLETED {:?}", tx_outcome.hash);
                        Ok(TransactionBatchOutcome::Single(tx_outcome))
                    }
                    // We sent a transaction message tagged as successful, so we can only get `TransactionIncluded` back
                    ConsumeMessageOutcome::BlockAdded
                    | ConsumeMessageOutcome::FailedTransactionIgnored => unreachable!(),
                }
            }
            Self::Batch {
                non_last_actions,
                last_action,
                ..
            } => {
                let mut non_last_outcomes = Vec::with_capacity(non_last_actions.len());
                let mut cumulative_diff = Diff::default();

                let block_hash = match last_action.as_ref() {
                    Some(tx_msg) => tx_msg.block_hash,
                    None => {
                        // This case should never come up because empty
                        // batches are thrown out before processing.
                        return Ok(TransactionBatchOutcome::Batch {
                            cumulative_diff: Diff::default(),
                            non_last_outcomes: Vec::new(),
                            last_outcome: None,
                        });
                    }
                };
                let block_height = storage.get_block_height_by_hash(block_hash)?;
                let block_metadata = storage.get_block_metadata(block_hash)?;
                let engine_account_id = storage.get_engine_account_id()?;
                // We need to use `BatchIO` here instead of simply calling `sync::consume_message` because
                // the latter no longer persists to the DB right away (we wait util checking the expected diff first now),
                // but a later transaction in a batch can see earlier ones, therefore we need to keep track all
                // changes made and expose them as if they had been committed to the DB.
                for tx in non_last_actions {
                    let transaction_position = tx.position;
                    let local_engine_account_id = engine_account_id.clone();
                    let (tx_hash, diff, result) = storage
                        .with_engine_access(
                            block_height,
                            transaction_position,
                            &tx.raw_input,
                            |io| {
                                let local_diff = RefCell::new(Diff::default());
                                let batch_io = BatchIO {
                                    fallback: io,
                                    cumulative_diff: &cumulative_diff,
                                    current_diff: &local_diff,
                                };
                                sync::execute_transaction::<_, M, _>(
                                    &tx,
                                    block_height,
                                    &block_metadata,
                                    local_engine_account_id,
                                    batch_io,
                                    |x| x.current_diff.borrow().clone(),
                                )
                            },
                        )
                        .result;
                    cumulative_diff.append(diff.clone());
                    let tx_outcome = TransactionIncludedOutcome {
                        hash: tx_hash,
                        info: tx,
                        diff,
                        maybe_result: result,
                    };
                    debug!("COMPLETED {:?}", tx_outcome.hash);
                    non_last_outcomes.push(tx_outcome);
                }
                let last_outcome = match last_action {
                    None => None,
                    Some(tx) => {
                        let transaction_position = tx.position;
                        let (tx_hash, diff, result) = storage
                            .with_engine_access(
                                block_height,
                                transaction_position,
                                &tx.raw_input,
                                |io| {
                                    let local_diff = RefCell::new(Diff::default());
                                    let batch_io = BatchIO {
                                        fallback: io,
                                        cumulative_diff: &cumulative_diff,
                                        current_diff: &local_diff,
                                    };
                                    sync::execute_transaction::<_, M, _>(
                                        &tx,
                                        block_height,
                                        &block_metadata,
                                        engine_account_id,
                                        batch_io,
                                        |x| x.current_diff.borrow().clone(),
                                    )
                                },
                            )
                            .result;
                        cumulative_diff.append(diff.clone());
                        let tx_outcome = TransactionIncludedOutcome {
                            hash: tx_hash,
                            info: tx,
                            diff,
                            maybe_result: result,
                        };
                        debug!("COMPLETED {:?}", tx_outcome.hash);
                        Some(Box::new(tx_outcome))
                    }
                };
                Ok(TransactionBatchOutcome::Batch {
                    cumulative_diff,
                    non_last_outcomes,
                    last_outcome,
                })
            }
        }
    }
}

enum TransactionBatchOutcome {
    Single(Box<TransactionIncludedOutcome>),
    Batch {
        cumulative_diff: Diff,
        non_last_outcomes: Vec<TransactionIncludedOutcome>,
        last_outcome: Option<Box<TransactionIncludedOutcome>>,
    },
}

impl TransactionBatchOutcome {
    const fn diff(&self) -> &Diff {
        match self {
            Self::Single(tx_outcome) => &tx_outcome.diff,
            Self::Batch {
                cumulative_diff, ..
            } => cumulative_diff,
        }
    }

    fn commit(&self, storage: &mut Storage) -> Result<(), engine_standalone_storage::Error> {
        match self {
            Self::Single(tx_outcome) => tx_outcome.commit(storage),
            Self::Batch {
                non_last_outcomes,
                last_outcome,
                ..
            } => {
                let all_outcomes = non_last_outcomes
                    .iter()
                    .chain(last_outcome.iter().map(|x| x.as_ref()));
                for tx_outcome in all_outcomes {
                    tx_outcome.commit(storage)?
                }
                Ok(())
            }
        }
    }

    fn update_diff(
        &self,
        storage: &mut Storage,
        expected_diff: &Diff,
    ) -> Result<(), engine_standalone_storage::Error> {
        match self {
            Self::Single(tx_outcome) => {
                storage.set_transaction_included(tx_outcome.hash, &tx_outcome.info, expected_diff)
            }
            Self::Batch {
                non_last_outcomes,
                last_outcome,
                ..
            } => {
                // It is awkward here because we want to be able to index each action in the batch
                // separately, but in the case of a diff mismatch we only have the expected diff of
                // the whole NEAR receipt (all actions together). We cannot reliably break this
                // cumulative diff into individual diffs, so instead we choose to associate the whole
                //  diff with the last action in the batch.

                // Note: this should always be `Some` because if `last_outcome` is empty then
                // `non_last_outcomes` will be non-empty (completely empty batches were thrown
                // out much earlier in the process).
                if let Some(tx_outcome) = last_outcome
                    .as_ref()
                    .map(|x| x.as_ref())
                    .or_else(|| non_last_outcomes.last())
                {
                    storage.set_transaction_included(
                        tx_outcome.hash,
                        &tx_outcome.info,
                        expected_diff,
                    )?
                }
                Ok(())
            }
        }
    }
}

fn parse_actions(
    actions: &[ActionView],
    promise_data: &[Option<Vec<u8>>],
) -> Option<ParsedActions> {
    let num_actions = actions.len();
    if num_actions == 1 {
        parse_action(&actions[0], promise_data).map(|(tx, input, n)| {
            ParsedActions::Single(SingleParsedAction {
                transaction_kind: Box::new(tx),
                raw_input: input,
                deposit: n,
            })
        })
    } else if num_actions == 0 {
        None
    } else {
        let last_index = num_actions - 1;
        let aurora_batch_elements: Vec<_> = actions
            .iter()
            .enumerate()
            .filter_map(|(i, action)| {
                parse_action(action, promise_data).map(|(tx, input, n)| {
                    let index = if i == last_index {
                        BatchIndex::Last(i as u32)
                    } else {
                        BatchIndex::Index(i as u32)
                    };

                    (
                        index,
                        SingleParsedAction {
                            transaction_kind: Box::new(tx),
                            raw_input: input,
                            deposit: n,
                        },
                    )
                })
            })
            .collect();
        if aurora_batch_elements.is_empty() {
            None
        } else {
            Some(ParsedActions::Batch(aurora_batch_elements))
        }
    }
}

/// Attempt to parse an Aurora transaction from the given NEAR action.
fn parse_action(
    action: &ActionView,
    promise_data: &[Option<Vec<u8>>],
) -> Option<(TransactionKind, Vec<u8>, u128)> {
    if let ActionView::FunctionCall {
        method_name,
        args,
        deposit,
        ..
    } = action
    {
        let bytes = args.to_vec();
        let transaction_kind =
            sync::parse_transaction_kind(method_name, bytes.clone(), promise_data).ok()?;
        return Some((transaction_kind, bytes, *deposit));
    }

    None
}
