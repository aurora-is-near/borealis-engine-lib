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
        ConsumeMessageOutcome, TransactionExecutionResult,
    },
    BlockMetadata, Diff, Storage,
};
use lru::LruCache;
use std::collections::HashMap;
use std::convert::TryFrom;
use tracing::{debug, warn};

pub fn consume_near_block(
    storage: &mut Storage,
    message: &aurora_refiner_types::near_block::NEARBlock,
    data_id_mapping: &mut LruCache<CryptoHash, Option<Vec<u8>>>,
    engine_account_id: &AccountId,
) -> Result<(), engine_standalone_storage::Error> {
    let block_hash = add_block_data_from_near_block(storage, message)?;

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

            let (signer, maybe_tx) = match &outcome.receipt.receipt {
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
                    // TODO: will we need to handle the case where multiple actions are relevant to Aurora?
                    let maybe_tx = actions.iter().find_map(|a| parse_action(a, &input_data));

                    (signer_id, maybe_tx)
                }
                near_primitives::views::ReceiptEnumView::Data { .. } => return None,
            };

            let signer = signer.as_ref().parse().ok()?;
            let caller = outcome.receipt.predecessor_id.as_ref().parse().ok()?;
            let near_receipt_id = outcome.receipt.receipt_id.0.into();
            let (transaction_kind, attached_near) = match maybe_tx {
                Some(tn) => tn,
                None => {
                    if expected_diffs.contains_key(&near_receipt_id) {
                        warn!(
                            "Receipt {:?} not parsed as transaction, but has state changes",
                            near_receipt_id,
                        );
                        (types::TransactionKind::Unknown, 0)
                    } else {
                        return None;
                    }
                }
            };

            // Ignore failed transactions since they do not impact the engine state
            let execution_result_bytes = match &outcome.execution_outcome.outcome.status {
                near_primitives::views::ExecutionStatusView::Unknown => return None,
                near_primitives::views::ExecutionStatusView::Failure(_) => return None,
                near_primitives::views::ExecutionStatusView::SuccessValue(bytes) => {
                    Some(base64::decode(bytes).ok()?)
                }
                near_primitives::views::ExecutionStatusView::SuccessReceiptId(_) => None,
            };

            let transaction_message = types::TransactionMessage {
                block_hash,
                near_receipt_id,
                position: position_counter,
                succeeded: true, // we drop failed transactions above
                signer,
                caller,
                attached_near,
                transaction: transaction_kind,
            };
            position_counter += 1;

            Some((transaction_message, execution_result_bytes))
        });

    for (t, result_bytes) in transaction_messages {
        let receipt_id = t.near_receipt_id;
        debug!("Processing receipt {:?}", receipt_id);
        let outcome = sync::consume_message(storage, Message::Transaction(Box::new(t)))?;
        if let ConsumeMessageOutcome::TransactionIncluded(tx_outcome) = outcome {
            debug!("COMPLETED {:?}", tx_outcome.hash);
            if let Ok(Some(TransactionExecutionResult::Submit(submit_result))) =
                &tx_outcome.maybe_result
            {
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
                    if !tx_outcome.diff.is_empty() {
                        warn!(
                            "Receipt {:?} not expected to have changes, but standalone computed diff {:?}",
                            receipt_id, tx_outcome.diff,
                        );
                        storage.revert_transaction_included(
                            tx_outcome.hash,
                            &tx_outcome.info,
                            &tx_outcome.diff,
                        )?;
                    }
                }
                Some(expected_diff) => {
                    if expected_diff != &tx_outcome.diff {
                        warn!(
                            "Diff mismatch in receipt_id={:?} computed={:?} ; expected={:?}",
                            receipt_id, tx_outcome.diff, expected_diff,
                        );
                        // Need to delete the incorrect diff before adding the correct diff because it could be
                        // the case that the incorrect diff wrote some keys that the correct diff did not
                        // (and these writes need to be undone).
                        storage.revert_transaction_included(
                            tx_outcome.hash,
                            &tx_outcome.info,
                            &tx_outcome.diff,
                        )?;
                        storage.set_transaction_included(
                            tx_outcome.hash,
                            &tx_outcome.info,
                            expected_diff,
                        )?;
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
) -> Result<H256, engine_standalone_storage::Error> {
    let block_message = types::BlockMessage {
        height: message.block.header.height,
        hash: message.block.header.hash.0.into(),
        metadata: BlockMetadata {
            timestamp: env::Timestamp::new(message.block.header.timestamp_nanosec),
            random_seed: message.block.header.random_value.0.into(),
        },
    };
    // TODO: Should covert to Aurora block hash?
    let block_hash = block_message.hash;

    debug!("Consuming block {}", block_message.height);
    sync::consume_message(storage, Message::Block(block_message))?;

    Ok(block_hash)
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
        let transaction_kind = match method_name.as_str() {
            "submit" => {
                let eth_tx = EthTransactionKind::try_from(bytes.as_slice()).ok()?;
                TransactionKind::Submit(eth_tx)
            }
            "call" => {
                let call_args = parameters::CallArgs::deserialize(&bytes)?;
                TransactionKind::Call(call_args)
            }
            "deploy_code" => TransactionKind::Deploy(bytes),
            "deploy_erc20_token" => {
                let deploy_args = parameters::DeployErc20TokenArgs::try_from_slice(&bytes).ok()?;
                TransactionKind::DeployErc20(deploy_args)
            }
            "ft_on_transfer" => {
                let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                let transfer_args = parameters::NEP141FtOnTransferArgs::try_from(json_args).ok()?;
                TransactionKind::FtOnTransfer(transfer_args)
            }
            "ft_transfer_call" => {
                let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                let transfer_args = parameters::TransferCallCallArgs::try_from(json_args).ok()?;
                TransactionKind::FtTransferCall(transfer_args)
            }
            "deposit" => TransactionKind::Deposit(bytes),
            "finish_deposit" => {
                let args = parameters::FinishDepositCallArgs::try_from_slice(&bytes).ok()?;
                TransactionKind::FinishDeposit(args)
            }
            "ft_resolve_transfer" => {
                let args = parameters::ResolveTransferCallArgs::try_from_slice(&bytes).ok()?;
                let promise_result = match promise_data.first().and_then(|x| x.as_ref()) {
                    Some(bytes) => {
                        aurora_engine_types::types::PromiseResult::Successful(bytes.clone())
                    }
                    None => aurora_engine_types::types::PromiseResult::Failed,
                };
                TransactionKind::ResolveTransfer(args, promise_result)
            }
            "ft_transfer" => {
                let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                let args = parameters::TransferCallArgs::try_from(json_args).ok()?;
                TransactionKind::FtTransfer(args)
            }
            "withdraw" => {
                let args =
                    aurora_engine_types::parameters::WithdrawCallArgs::try_from_slice(&bytes)
                        .ok()?;
                TransactionKind::Withdraw(args)
            }
            "storage_deposit" => {
                let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                let args = parameters::StorageDepositCallArgs::from(json_args);
                TransactionKind::StorageDeposit(args)
            }
            "storage_unregister" => {
                let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                let force = json_args.bool("force").ok();
                TransactionKind::StorageUnregister(force)
            }
            "storage_withdraw" => {
                let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                let args = parameters::StorageWithdrawCallArgs::from(json_args);
                TransactionKind::StorageWithdraw(args)
            }
            "set_paused_flags" => {
                let args = parameters::PauseEthConnectorCallArgs::try_from_slice(&bytes).ok()?;
                TransactionKind::SetPausedFlags(args)
            }
            "register_relayer" => {
                let address = Address::try_from_slice(&bytes).ok()?;
                TransactionKind::RegisterRelayer(address)
            }
            "refund_on_error" => match promise_data.first().and_then(|x| x.as_ref()) {
                None => TransactionKind::RefundOnError(None),
                Some(_) => {
                    let args =
                        aurora_engine_types::parameters::RefundCallArgs::try_from_slice(&bytes)
                            .ok()?;
                    TransactionKind::RefundOnError(Some(args))
                }
            },
            "set_eth_connector_contract_data" => {
                let args = parameters::SetContractDataCallArgs::try_from_slice(&bytes).ok()?;
                TransactionKind::SetConnectorData(args)
            }
            "new_eth_connector" => {
                let args = parameters::InitCallArgs::try_from_slice(&bytes).ok()?;
                TransactionKind::NewConnector(args)
            }
            "new" => {
                let args = parameters::NewCallArgs::try_from_slice(&bytes).ok()?;
                TransactionKind::NewEngine(args)
            }
            _ => return None,
        };
        return Some((transaction_kind, *deposit));
    }

    None
}
