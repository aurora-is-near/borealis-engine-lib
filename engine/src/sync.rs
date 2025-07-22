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
use crate::log_file;

/// Helper function to decode and analyze storage keys/values for debugging
fn debug_storage_entry(key: &[u8], value: &[u8]) -> String {
    // Analyze the key structure
    if key.is_empty() {
        return "Empty key".to_string();
    }

    let mut result = String::new();

    let version = key[0];
    result.push_str(&format!("Key[v{}]: ", version));

    // Try to decode Aurora storage key structure
    if key.len() > 1 {
        // Key prefix is the second byte that indicates the type of Aurora storage entry
        // Common prefixes: 4=Storage, 5=Code, 6=Nonce, 7=Balance, etc.
        let key_prefix = key.get(1).copied().unwrap_or(0);
        result.push_str(&format!("prefix({}) ", key_prefix));

        // Decode common Aurora key structures
        if key.len() >= 22 && (key_prefix == 4 || key_prefix == 5) {
            // Likely Storage or Code keys
            // Extract address (20 bytes after version and prefix)
            let address_bytes = &key[2..22];
            result.push_str(&format!("addr(0x{}) ", hex::encode(address_bytes)));

            // If there's more data, it might be a storage slot
            if key.len() > 22 {
                let slot_data = &key[22..];
                if slot_data.len() == 32 {
                    // 32-byte storage slot
                    result.push_str(&format!("slot(0x{}) ", hex::encode(&slot_data[..8])));
                } else if slot_data.len() == 36 {
                    // 32-byte slot + 4-byte generation
                    let generation = u32::from_le_bytes([
                        slot_data[32],
                        slot_data[33],
                        slot_data[34],
                        slot_data[35],
                    ]);
                    result.push_str(&format!(
                        "slot(0x{}) gen({}) ",
                        hex::encode(&slot_data[..8]),
                        generation
                    ));
                } else {
                    result.push_str(&format!("extra({} bytes) ", slot_data.len()));
                }
            }
        } else {
            // Unknown structure, show raw bytes
            let key_suffix = &key[1..];
            result.push_str(&format!(
                "raw({}) ",
                hex::encode(&key_suffix[..key_suffix.len().min(16)])
            ));
        }

        // Look for ASCII strings in key
        if let Some(ascii) = extract_ascii_strings(&key[1..]) {
            if !ascii.is_empty() {
                result.push_str(&format!("key_strings:[{}] ", ascii.join(", ")));
            }
        }
    }

    // Analyze the value
    result.push_str(&format!("\nValue({} bytes): ", value.len()));

    if value.len() <= 32 {
        // Small values - show as hex and try as number
        result.push_str(&format!("0x{}", hex::encode(value)));
        if value.len() <= 8 && !value.is_empty() {
            if let Some(num) = bytes_to_number(value) {
                result.push_str(&format!(" ({})", num));
            }
        }
    } else {
        // Large values - likely bytecode
        result.push_str(&format!(
            "{}...{}",
            hex::encode(&value[..16.min(value.len())]),
            hex::encode(&value[value.len().saturating_sub(16)..])
        ));

        // For bytecode, try to identify if it's contract code
        if value.len() > 100 {
            // Look for common Solidity patterns
            let mut patterns = Vec::new();

            // Check for constructor signature
            if value.starts_with(&[0x60, 0x80]) {
                patterns.push("Solidity_Constructor");
            }

            // Check for function selector patterns
            if value.windows(4).any(|w| w == [0x63, 0x77, 0xd3, 0x2e]) {
                // common selector pattern
                patterns.push("Function_Selectors");
            }

            if !patterns.is_empty() {
                result.push_str(&format!(" patterns:[{}]", patterns.join(", ")));
            }
        }

        // Extract ASCII strings from bytecode/data
        if let Some(strings) = extract_ascii_strings(value) {
            if !strings.is_empty() {
                let important_strings: Vec<_> = strings
                    .into_iter()
                    .filter(|s| {
                        s.len() > 3
                            && (s.contains("Transaction")
                                || s.contains("Value")
                                || s.contains("Event")
                                || s.contains("Function")
                                || s.contains("Contract")
                                || s.contains("Error")
                                || s.len() > 8)
                    })
                    .take(5) // Limit to 5 strings to prevent log spam from large bytecode
                    .collect();

                if !important_strings.is_empty() {
                    result.push_str(&format!(" strings:[{}]", important_strings.join(", ")));
                }
            }
        }
    }

    result
}

/// Extract ASCII strings from byte data (minimum 4 chars, printable only)
fn extract_ascii_strings(data: &[u8]) -> Option<Vec<String>> {
    let mut strings = Vec::new();
    let mut current_string = String::new();

    for &byte in data {
        if byte >= 32 && byte <= 126 {
            // Printable ASCII
            current_string.push(byte as char);
        } else {
            if current_string.len() >= 4 {
                // Minimum 4 characters
                strings.push(current_string.clone());
            }
            current_string.clear();
        }
    }

    // Don't forget the last string
    if current_string.len() >= 4 {
        strings.push(current_string);
    }

    if strings.is_empty() {
        None
    } else {
        Some(strings)
    }
}

/// Convert bytes to number (little-endian)
fn bytes_to_number(bytes: &[u8]) -> Option<u64> {
    match bytes.len() {
        1 => Some(bytes[0] as u64),
        2 => Some(u16::from_le_bytes([bytes[0], bytes[1]]) as u64),
        4 => Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64),
        8 => Some(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])),
        _ => None,
    }
}

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

    if message.block.header.height == 42384870 {
        warn!("debug");
    }

    // Capture data receipts (for using in promises). Also, we create a mapping here because the
    // order of the `receipts` and `receipt_execution_outcomes` is different (probably BUG) and we
    // need to handle the behavior.
    let receipt_mapping = message
        .shards
        .iter()
        .filter_map(|shard| shard.chunk.as_ref())
        .flat_map(|chunk| chunk.receipts.as_slice())
        .enumerate()
        .filter_map(|(i, r)| {
            if r.receiver_id.as_str() == engine_account_id.as_ref() {
                if let near_primitives::views::ReceiptEnumView::Data { data_id, data, .. } =
                    &r.receipt
                {
                    data_id_mapping.put(*data_id, data.clone());
                }

                Some((r.receipt_id, i))
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>();

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
            aurora_refiner_types::near_block::StateChangeCauseView::ReceiptProcessing {
                receipt_hash,
            } => receipt_hash.0.into(),
            other => panic!("Unexpected state change cause {:?}", other),
        };

        let diff = expected_diffs.entry(receipt_id).or_default();
        match expected_value {
            Some(value) => diff.modify(key.to_vec(), value.to_vec()),
            None => diff.delete(key.to_vec()),
        }
    }

    let mut position_counter = 0;
    let mut receipt_execution_outcomes = message
        .shards
        .iter()
        .flat_map(|shard| shard.receipt_execution_outcomes.iter())
        .filter(|o| o.receipt.receiver_id.as_str() == engine_account_id.as_ref())
        .collect::<Vec<_>>();

    receipt_execution_outcomes.sort_by_key(|item| {
        receipt_mapping
            .get(&item.receipt.receipt_id)
            .copied()
            .unwrap_or_else(|| {
                warn!("Receipt {:?} not found in mapping", item.receipt.receipt_id);
                usize::MAX
            })
    });

    let transaction_messages = receipt_execution_outcomes.iter().filter_map(|outcome| {
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
            near_primitives::views::ReceiptEnumView::GlobalContractDistribution { .. } => {
                return None;
            }
        };

        let signer = signer.as_str().parse().ok()?;
        let caller = outcome.receipt.predecessor_id.as_str().parse().ok()?;
        let near_receipt_id = outcome.receipt.receipt_id.0.into();
        let maybe_batch_actions = match maybe_tx {
            Some(tn) => tn,
            None => {
                if expected_diffs.contains_key(&near_receipt_id) {
                    // Log the method names that failed to parse
                    let method_names: Vec<String> = if let near_primitives::views::ReceiptEnumView::Action { actions, .. } = &outcome.receipt.receipt {
                        actions.iter().filter_map(|action| {
                            if let near_primitives::views::ActionView::FunctionCall { method_name, .. } = action {
                                Some(method_name.clone())
                            } else {
                                None
                            }
                        }).collect()
                    } else {
                        Vec::new()
                    };
                    let block_height = message.block.header.height;
                    let expected_diff_count = expected_diffs.get(&near_receipt_id).map(|d| d.iter().count()).unwrap_or(0);
                    crate::warn_and_log_to_file!(
                        "consume_near_block {} :: Receipt {:?} not parsed as transaction, but has state changes. Methods: {:?}, Expected diff count: {}",
                        block_height, near_receipt_id, method_names, expected_diff_count
                    );
                    ParsedActions::Single(SingleParsedAction::default()) // <- TransactionKind::Unknown
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
                let computed_diff = tx_outcome.diff();
                if expected_diff == computed_diff {
                    // Diff was correct, so commit it to the storage
                    tx_outcome.commit(storage)?;
                } else {
                    let diff_mismatch_count = log_file::diff_mismatch_increment() + 1;
                    let block_height = message.block.header.height;

                    crate::log_to_file!(
                        "=== consume_near_block {} :: START :: {} ===\n",
                        block_height,
                        diff_mismatch_count
                    );

                    // Diff was incorrect, so log a warning and commit
                    // the one from the Near block instead
                    warn!(
                        "Receipt {receipt_id:?} diff mismatch with computed diff,\nblock height: {:?},\nexpected diff count: {:?},\ncomputed diff count: {:?}",
                        message.block.header.height,
                        expected_diff.iter().count(),
                        computed_diff.iter().count()
                    );

                    log_file::log_to_file!(
                        "Receipt {receipt_id:?} diff mismatch with computed diff,\nblock height: {:?},\nexpected diff count: {:?},\nexpected diff: {:?},\ncomputed diff count: {:?},\ncomputed diff: {:?}\n",
                        message.block.header.height,
                        expected_diff.iter().count(),
                        expected_diff,
                        computed_diff.iter().count(),
                        computed_diff
                    );

                    // Debug: Analyze the expected diff entries
                    crate::log_to_file!("EXPECTED DIFF :: START");
                    for (key, diff_op) in expected_diff.iter() {
                        match diff_op.value() {
                            Some(value) => {
                                crate::log_to_file!(
                                    "EXPECTED MODIFIED:\n{}",
                                    debug_storage_entry(&key, &value)
                                );
                            }
                            None => {
                                crate::log_to_file!(
                                    "EXPECTED DELETE:\nKey[v{}]: ({})",
                                    key.get(0).unwrap_or(&0),
                                    hex::encode(&key[..key.len().min(16)])
                                );
                            }
                        }
                    }
                    crate::log_to_file!("EXPECTED DIFF :: END");

                    // Debug: Analyze the computed diff entries
                    crate::log_to_file!("COMPUTED DIFF :: START");
                    for (key, diff_op) in computed_diff.iter() {
                        match diff_op.value() {
                            Some(value) => {
                                crate::log_to_file!(
                                    "COMPUTED MODIFIED:\n{}",
                                    debug_storage_entry(&key, &value)
                                );
                            }
                            None => {
                                crate::log_to_file!(
                                    "COMPUTED DELETE:\nKey[v{}]: ({})",
                                    key.get(0).unwrap_or(&0),
                                    hex::encode(&key[..key.len().min(16)])
                                );
                            }
                        }
                    }

                    crate::log_to_file!("COMPUTED DIFF :: END");

                    // The engine state is correct according to Near, so we commit the expected diff
                    tx_outcome.update_diff(storage, expected_diff)?;

                    crate::log_to_file!("\n=== consume_near_block {} :: END ===\n\n", block_height);
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

    if block_message.height % 10000 == 0 {
        debug!("Consuming block {}", block_message.height);
    }
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
