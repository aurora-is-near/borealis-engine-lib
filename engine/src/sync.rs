use aurora_engine::parameters;
use aurora_engine_sdk::env;
use aurora_engine_transactions::EthTransactionKind;
use aurora_engine_types::{account_id::AccountId, types::Address, H256};
use aurora_refiner_types::near_primitives::{
    self,
    hash::CryptoHash,
    views::{ActionView, StateChangeValueView},
};
use borsh::BorshDeserialize;
use engine_standalone_storage::sync::types::TransactionKind;
use engine_standalone_storage::{
    sync::{
        self,
        types::{self, Message},
        ConsumeMessageOutcome, TransactionExecutionResult, TransactionIncludedOutcome,
    },
    BlockMetadata, Diff, Storage,
};
use lru::LruCache;
use std::convert::TryFrom;
use std::{collections::HashMap, str::FromStr};
use tracing::{debug, warn};

use crate::types::InnerTransactionKind;

pub fn consume_near_block(
    storage: &mut Storage,
    message: &aurora_refiner_types::near_block::NEARBlock,
    data_id_mapping: &mut LruCache<CryptoHash, Option<Vec<u8>>>,
    engine_account_id: &AccountId,
    chain_id: [u8; 32],
    mut outcomes: Option<&mut HashMap<H256, TransactionIncludedOutcome>>,
) -> Result<(), engine_standalone_storage::Error> {
    let block_hash = add_block_data_from_near_block(storage, message, chain_id, engine_account_id)?;

    // Capture data receipts (for using in promises)
    message
        .shards
        .iter()
        .filter_map(|shard| shard.chunk.as_ref())
        .flat_map(|chunk| chunk.receipts.as_slice())
        .for_each(|r| {
            if r.receiver_id.as_ref() == engine_account_id.as_ref() {
                if let near_primitives::views::ReceiptEnumView::Data { data_id, data } = &r.receipt
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
        .filter(|(account_id, _, _, _)| account_id.as_ref() == engine_account_id.as_ref());

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
            Some(value) => diff.modify(to_vec(key), to_vec(value)),
            None => diff.delete(to_vec(key)),
        }
    }

    let mut position_counter = 0;
    let transaction_messages = message
        .shards
        .iter()
        .flat_map(|shard| shard.receipt_execution_outcomes.iter())
        .filter_map(|outcome| {
            if outcome.receipt.receiver_id.as_ref() != engine_account_id.as_ref() {
                return None;
            }

            // Ignore failed transactions since they do not impact the engine state
            let execution_result_bytes = match &outcome.execution_outcome.outcome.status {
                near_primitives::views::ExecutionStatusView::Unknown => return None,
                near_primitives::views::ExecutionStatusView::Failure(_) => return None,
                near_primitives::views::ExecutionStatusView::SuccessValue(bytes) => {
                    Some(base64::decode(bytes).ok()?)
                }
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

            let signer = signer.as_ref().parse().ok()?;
            let caller = outcome.receipt.predecessor_id.as_ref().parse().ok()?;
            let near_receipt_id = outcome.receipt.receipt_id.0.into();
            let maybe_batch_actions = match maybe_tx {
                Some(tn) => tn,
                None => {
                    if expected_diffs.contains_key(&near_receipt_id) {
                        warn!(
                            "Receipt {:?} not parsed as transaction, but has state changes",
                            near_receipt_id,
                        );
                        ParsedActions::Single(Box::new(types::TransactionKind::Unknown), 0)
                    } else {
                        return None;
                    }
                }
            };

            let transaction_messages = match maybe_batch_actions {
                ParsedActions::Single(transaction_kind, attached_near) => {
                    let transaction_message = types::TransactionMessage {
                        block_hash,
                        near_receipt_id,
                        position: position_counter,
                        succeeded: true, // we drop failed transactions above
                        signer,
                        caller,
                        attached_near,
                        transaction: *transaction_kind,
                        promise_data,
                    };
                    position_counter += 1;

                    TransactionBatch::Single(transaction_message)
                }

                ParsedActions::Batch(txs) => {
                    let mut non_last_actions: Vec<_> = txs
                        .into_iter()
                        .map(|(index, transaction_kind, attached_near)| {
                            let virtual_receipt_id = match index {
                                BatchIndex::Index(i) => {
                                    let mut bytes = [0u8; 36];
                                    bytes[0..32].copy_from_slice(near_receipt_id.as_bytes());
                                    bytes[32..36].copy_from_slice(&i.to_be_bytes());
                                    aurora_refiner_types::utils::keccak256(&bytes)
                                }
                                BatchIndex::Last => near_receipt_id,
                            };
                            let transaction_message = types::TransactionMessage {
                                block_hash,
                                near_receipt_id: virtual_receipt_id,
                                position: position_counter,
                                succeeded: true, // we drop failed transactions above
                                signer: signer.clone(),
                                caller: caller.clone(),
                                attached_near,
                                transaction: transaction_kind,
                                promise_data: promise_data.clone(),
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
        let tx_outcome = t.process(storage)?;
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
                                    "Incorrect result in processing receipt_id={:?} computed={:?} expected={:?}",
                                    receipt_id,
                                    submit_result,
                                    expected_result,
                                );
                            }
                        }
                        Err(_) => warn!(
                            "Unable to deserialize receipt_id={:?} as SubmitResult",
                            receipt_id
                        ),
                    }
                }
                None => warn!(
                    "Expected receipt_id={:?} to have a return result, but there was none",
                    receipt_id
                ),
            }
        }
        // Validate against expected diff
        match expected_diffs.get(&receipt_id) {
            None => {
                if !tx_outcome.diff().is_empty() {
                    warn!(
                        "Receipt {:?} not expected to have changes, but standalone computed diff {:?}",
                        receipt_id, tx_outcome.diff(),
                    );
                    tx_outcome.revert(storage)?;
                }
            }
            Some(expected_diff) => {
                if expected_diff != tx_outcome.diff() {
                    warn!(
                        "Diff mismatch in receipt_id={:?} computed={:?} ; expected={:?}",
                        receipt_id,
                        tx_outcome.diff(),
                        expected_diff,
                    );
                    // Need to delete the incorrect diff before adding the correct diff because it could be
                    // the case that the incorrect diff wrote some keys that the correct diff did not
                    // (and these writes need to be undone).
                    tx_outcome.revert(storage)?;
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

fn to_vec<T: AsRef<[u8]>>(t: T) -> Vec<u8> {
    t.as_ref().to_vec()
}

fn add_block_data_from_near_block(
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
    sync::consume_message(storage, Message::Block(block_message))?;

    Ok(block_hash)
}

/// We treat the last element of a batch differently from the rest because its outcome is the outcome
/// of the whole receipt. This enum tags the elements of a batch for downstream processing.
enum BatchIndex {
    Index(u32),
    Last,
}

/// Most NEAR receipts are not batches, so we want to optimize for the case where there is just one
/// action (not allocate a vec every time). This enum enables that optimization.
enum ParsedActions {
    Single(Box<TransactionKind>, u128),
    Batch(Vec<(BatchIndex, TransactionKind, u128)>),
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
    fn near_receipt_id(&self) -> H256 {
        match self {
            Self::Single(tx) => tx.near_receipt_id,
            Self::Batch {
                near_receipt_id, ..
            } => *near_receipt_id,
        }
    }

    fn process(
        self,
        storage: &mut Storage,
    ) -> Result<TransactionBatchOutcome, engine_standalone_storage::Error> {
        match self {
            Self::Single(tx) => {
                match sync::consume_message(storage, Message::Transaction(Box::new(tx)))? {
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
                for tx in non_last_actions {
                    match sync::consume_message(storage, Message::Transaction(Box::new(tx)))? {
                        ConsumeMessageOutcome::TransactionIncluded(tx_outcome) => {
                            debug!("COMPLETED {:?}", tx_outcome.hash);
                            non_last_outcomes.push(*tx_outcome);
                        }
                        // We sent a transaction message tagged as successful, so we can only get `TransactionIncluded` back
                        ConsumeMessageOutcome::BlockAdded
                        | ConsumeMessageOutcome::FailedTransactionIgnored => unreachable!(),
                    }
                }
                let last_outcome = match last_action {
                    None => None,
                    Some(tx) => {
                        match sync::consume_message(storage, Message::Transaction(Box::new(tx)))? {
                            ConsumeMessageOutcome::TransactionIncluded(tx_outcome) => {
                                debug!("COMPLETED {:?}", tx_outcome.hash);
                                Some(tx_outcome)
                            }
                            ConsumeMessageOutcome::BlockAdded
                            | ConsumeMessageOutcome::FailedTransactionIgnored => unreachable!(),
                        }
                    }
                };
                let cumulative_diff = non_last_outcomes
                    .iter()
                    .chain(last_outcome.iter().map(|x| x.as_ref()))
                    .fold(Diff::default(), |mut acc, outcome| {
                        acc.append(outcome.diff.clone());
                        acc
                    });
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
    fn diff(&self) -> &Diff {
        match self {
            Self::Single(tx_outcome) => &tx_outcome.diff,
            Self::Batch {
                cumulative_diff, ..
            } => cumulative_diff,
        }
    }

    fn revert(&self, storage: &mut Storage) -> Result<(), engine_standalone_storage::Error> {
        match self {
            Self::Single(tx_outcome) => storage.revert_transaction_included(
                tx_outcome.hash,
                &tx_outcome.info,
                &tx_outcome.diff,
            ),
            Self::Batch {
                non_last_outcomes,
                last_outcome,
                ..
            } => {
                let all_outcomes = non_last_outcomes
                    .iter()
                    .chain(last_outcome.iter().map(|x| x.as_ref()));
                for tx_outcome in all_outcomes {
                    storage.revert_transaction_included(
                        tx_outcome.hash,
                        &tx_outcome.info,
                        &tx_outcome.diff,
                    )?
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
        parse_action(&actions[0], promise_data)
            .map(|(tx, n)| ParsedActions::Single(Box::new(tx), n))
    } else {
        let last_index = num_actions - 1;
        let aurora_batch_elements: Vec<_> = actions
            .iter()
            .enumerate()
            .filter_map(|(i, action)| {
                parse_action(action, promise_data).map(|(tx, n)| {
                    let index = if i == last_index {
                        BatchIndex::Last
                    } else {
                        BatchIndex::Index(i as u32)
                    };

                    (index, tx, n)
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
) -> Option<(TransactionKind, u128)> {
    if let ActionView::FunctionCall {
        method_name,
        args,
        deposit,
        ..
    } = action
    {
        let bytes = base64::decode(&args).ok()?;

        let transaction_kind = if let Ok(raw_tx_kind) =
            InnerTransactionKind::from_str(method_name.as_str())
        {
            match raw_tx_kind {
                InnerTransactionKind::Submit => {
                    let eth_tx = EthTransactionKind::try_from(bytes.as_slice()).ok()?;
                    TransactionKind::Submit(eth_tx)
                }
                InnerTransactionKind::Call => {
                    let call_args = parameters::CallArgs::deserialize(&bytes)?;
                    TransactionKind::Call(call_args)
                }
                InnerTransactionKind::Deploy => TransactionKind::Deploy(bytes),
                InnerTransactionKind::DeployErc20 => {
                    let deploy_args =
                        parameters::DeployErc20TokenArgs::try_from_slice(&bytes).ok()?;
                    TransactionKind::DeployErc20(deploy_args)
                }
                InnerTransactionKind::FtOnTransfer => {
                    let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                    let transfer_args =
                        parameters::NEP141FtOnTransferArgs::try_from(json_args).ok()?;
                    TransactionKind::FtOnTransfer(transfer_args)
                }
                InnerTransactionKind::Deposit => TransactionKind::Deposit(bytes),
                InnerTransactionKind::FtTransferCall => {
                    let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                    let transfer_args =
                        parameters::TransferCallCallArgs::try_from(json_args).ok()?;
                    TransactionKind::FtTransferCall(transfer_args)
                }
                InnerTransactionKind::FinishDeposit => {
                    let args = parameters::FinishDepositCallArgs::try_from_slice(&bytes).ok()?;
                    TransactionKind::FinishDeposit(args)
                }
                InnerTransactionKind::ResolveTransfer => {
                    let args = parameters::ResolveTransferCallArgs::try_from_slice(&bytes).ok()?;
                    let promise_result = match promise_data.first().and_then(|x| x.as_ref()) {
                        Some(bytes) => {
                            aurora_engine_types::types::PromiseResult::Successful(bytes.clone())
                        }
                        None => aurora_engine_types::types::PromiseResult::Failed,
                    };
                    TransactionKind::ResolveTransfer(args, promise_result)
                }
                InnerTransactionKind::FtTransfer => {
                    let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                    let args = parameters::TransferCallArgs::try_from(json_args).ok()?;
                    TransactionKind::FtTransfer(args)
                }
                InnerTransactionKind::Withdraw => {
                    let args =
                        aurora_engine_types::parameters::WithdrawCallArgs::try_from_slice(&bytes)
                            .ok()?;
                    TransactionKind::Withdraw(args)
                }
                InnerTransactionKind::StorageDeposit => {
                    let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                    let args = parameters::StorageDepositCallArgs::from(json_args);
                    TransactionKind::StorageDeposit(args)
                }
                InnerTransactionKind::StorageUnregister => {
                    let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                    let force = json_args.bool("force").ok();
                    TransactionKind::StorageUnregister(force)
                }
                InnerTransactionKind::StorageWithdraw => {
                    let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                    let args = parameters::StorageWithdrawCallArgs::from(json_args);
                    TransactionKind::StorageWithdraw(args)
                }
                InnerTransactionKind::SetPausedFlags => {
                    let args =
                        parameters::PauseEthConnectorCallArgs::try_from_slice(&bytes).ok()?;
                    TransactionKind::SetPausedFlags(args)
                }
                InnerTransactionKind::RegisterRelayer => {
                    let address = Address::try_from_slice(&bytes).ok()?;
                    TransactionKind::RegisterRelayer(address)
                }
                InnerTransactionKind::RefundOnError => match promise_data
                    .first()
                    .and_then(|x| x.as_ref())
                {
                    None => TransactionKind::RefundOnError(None),
                    Some(_) => {
                        let args =
                            aurora_engine_types::parameters::RefundCallArgs::try_from_slice(&bytes)
                                .ok()?;
                        TransactionKind::RefundOnError(Some(args))
                    }
                },
                InnerTransactionKind::SetConnectorData => {
                    let args = parameters::SetContractDataCallArgs::try_from_slice(&bytes).ok()?;
                    TransactionKind::SetConnectorData(args)
                }
                InnerTransactionKind::NewConnector => {
                    let args = parameters::InitCallArgs::try_from_slice(&bytes).ok()?;
                    TransactionKind::NewConnector(args)
                }
                InnerTransactionKind::NewEngine => {
                    let args = parameters::NewCallArgs::try_from_slice(&bytes).ok()?;
                    TransactionKind::NewEngine(args)
                }
                InnerTransactionKind::FactoryUpdate => TransactionKind::FactoryUpdate(bytes),
                InnerTransactionKind::FactoryUpdateAddressVersion => {
                    let args = aurora_engine::xcc::AddressVersionUpdateArgs::try_from_slice(&bytes)
                        .ok()?;
                    TransactionKind::FactoryUpdateAddressVersion(args)
                }
                InnerTransactionKind::FactorySetWNearAddress => {
                    let address = Address::try_from_slice(&bytes).ok()?;
                    TransactionKind::FactorySetWNearAddress(address)
                }
                InnerTransactionKind::Unknown => {
                    warn!("Unknown method name: {}", method_name);
                    return None;
                }
            }
        } else {
            warn!("Unknown method name: {}", method_name);
            return None;
        };

        return Some((transaction_kind, *deposit));
    }

    None
}
