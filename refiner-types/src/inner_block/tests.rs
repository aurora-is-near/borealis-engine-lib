use super::*;
use near_primitives::{types::Balance, views};

const STREAMER_MESSAGE: &str =
    include_str!("../../tests/res/streamer_message_190534818_branch_remove_custom_indexer.json");
const HISTORICAL_BLOCK: &str = include_str!("../../tests/res/block_190534818_branch_main.json");
const HISTORICAL_BLOCK_V2: &str =
    include_str!("../../tests/res/block_190534818_branch_remove_custom_indexer.json");
const MAINNET_BLOCK: &str = include_str!("../../tests/res/near_block/block_151768255_mainnet.json");

fn source_message() -> near_indexer::StreamerMessage {
    serde_json::from_str(STREAMER_MESSAGE).unwrap()
}

#[test]
fn deserializes_historical_nearcore() {
    let expected = InnerNearBlock::try_from(source_message()).unwrap();

    for json in [HISTORICAL_BLOCK, HISTORICAL_BLOCK_V2] {
        let from_bytes = InnerNearBlock::from_bytes(json.as_bytes()).unwrap();
        assert_eq!(from_bytes, expected);
    }

    let mainnet_from_bytes = InnerNearBlock::from_bytes(MAINNET_BLOCK.as_bytes()).unwrap();
    assert_eq!(mainnet_from_bytes.block.header.height, 151_768_255);
}

#[test]
fn nearcore_and_historical_inputs_produce_the_same_internal_block() {
    let expected = InnerNearBlock::try_from(source_message()).unwrap();

    let historical = InnerNearBlock::from_bytes(HISTORICAL_BLOCK.as_bytes()).unwrap();
    let historical_v2 = InnerNearBlock::from_bytes(HISTORICAL_BLOCK_V2.as_bytes()).unwrap();

    assert_eq!(historical, expected);
    assert_eq!(historical_v2, expected);
}

#[test]
fn internal_block_has_symmetric_serde() {
    let expected = InnerNearBlock::try_from(source_message()).unwrap();
    let json = serde_json::to_vec(&expected).unwrap();
    let actual: InnerNearBlock = serde_json::from_slice(&json).unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn preserves_custom_action_bytes_receipt_size_and_action_positions() {
    let mut source = source_message();
    source.block.header.timestamp = 123;
    source.block.header.timestamp_nanosec = 456;
    let outcome = source
        .shards
        .iter_mut()
        .flat_map(|shard| &mut shard.receipt_execution_outcomes)
        .find(|outcome| {
            matches!(
                outcome.receipt.receipt,
                views::ReceiptEnumView::Action { .. }
            )
        })
        .unwrap();
    let views::ReceiptEnumView::Action { actions, .. } = &mut outcome.receipt.receipt else {
        unreachable!()
    };
    *actions = vec![
        views::ActionView::Transfer {
            deposit: Balance::from_yoctonear(42),
        },
        views::ActionView::FunctionCall {
            method_name: "submit".into(),
            args: vec![1, 2, 3].into(),
            gas: near_primitives::types::Gas::from_gas(100),
            deposit: Balance::from_yoctonear(u128::MAX),
        },
        views::ActionView::CreateAccount,
        views::ActionView::UniversalStateInit {
            state_init: near_primitives::universal_state_init::RawStateInit(vec![4, 5, 6]),
            deposit: Balance::from_yoctonear(7),
        },
    ];
    outcome.receipt._priority = 7;
    let id = outcome.receipt.receipt_id;
    let expected_size = borsh::to_vec(&outcome.receipt).unwrap().len() as u64;
    let failure = near_primitives::errors::TxExecutionError::InvalidTxError(
        near_primitives::errors::InvalidTxError::Expired,
    );
    let expected_failure = borsh::to_vec(&failure).unwrap();
    outcome.execution_outcome.outcome.status = views::ExecutionStatusView::Failure(failure);

    let bytes = serde_json::to_vec(&source).unwrap();
    let block = InnerNearBlock::from_bytes(&bytes).unwrap();
    assert_eq!(block, InnerNearBlock::try_from(source).unwrap());
    assert_eq!(block.block.header.timestamp, 123);
    assert_eq!(block.block.header.timestamp_nanosec, 456);
    let outcome = block
        .shards
        .iter()
        .flat_map(|shard| &shard.receipt_execution_outcomes)
        .find(|outcome| outcome.receipt.receipt_id == id)
        .unwrap();
    assert_eq!(outcome.receipt_size, Some(expected_size));
    assert_eq!(
        outcome.execution_outcome.status,
        ExecutionStatus::Failure(expected_failure)
    );
    let ReceiptKind::Action { actions, .. } = &outcome.receipt.receipt else {
        panic!("Expected action receipt");
    };
    let transfer_bytes = [vec![3], 42u128.to_le_bytes().to_vec()].concat();
    assert_eq!(
        actions,
        &vec![
            Action::Other {
                borsh_bytes: transfer_bytes
            },
            Action::FunctionCall {
                method_name: "submit".into(),
                args: vec![1, 2, 3],
                deposit: u128::MAX,
            },
            Action::Other {
                borsh_bytes: vec![0]
            },
            Action::Other {
                borsh_bytes: borsh::to_vec(&views::ActionView::UniversalStateInit {
                    state_init: near_primitives::universal_state_init::RawStateInit(vec![4, 5, 6]),
                    deposit: Balance::from_yoctonear(7),
                })
                .unwrap(),
            },
        ]
    );
}

#[test]
fn delegate_action_codec_matches_pinned_nearcore_borsh() {
    let source: near_indexer::StreamerMessage = serde_json::from_str(MAINNET_BLOCK).unwrap();
    let (receipt_id, action_index, expected_action, expected_size) = source
        .shards
        .iter()
        .flat_map(|shard| &shard.receipt_execution_outcomes)
        .find_map(|outcome| {
            let views::ReceiptEnumView::Action { actions, .. } = &outcome.receipt.receipt else {
                return None;
            };
            let index = actions
                .iter()
                .position(|action| matches!(action, views::ActionView::Delegate { .. }))?;
            Some((
                outcome.receipt.receipt_id,
                index,
                borsh::to_vec(&actions[index]).unwrap(),
                borsh::to_vec(&outcome.receipt).unwrap().len() as u64,
            ))
        })
        .unwrap();
    let block = InnerNearBlock::from_bytes(MAINNET_BLOCK.as_bytes()).unwrap();
    assert_eq!(block, InnerNearBlock::try_from(source).unwrap());
    let converted = block
        .shards
        .iter()
        .flat_map(|shard| &shard.receipt_execution_outcomes)
        .find(|outcome| outcome.receipt.receipt_id == receipt_id)
        .unwrap();
    assert_eq!(converted.receipt_size, Some(expected_size));
    let ReceiptKind::Action { actions, .. } = &converted.receipt.receipt else {
        panic!("Expected action receipt");
    };
    assert_eq!(
        actions[action_index],
        Action::Other {
            borsh_bytes: expected_action
        }
    );
}

#[test]
fn future_delegate_payload_in_external_receipt_remains_opaque() {
    let mut source: serde_json::Value = serde_json::from_str(MAINNET_BLOCK).unwrap();
    let outcome = source["shards"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|shard| shard["receipt_execution_outcomes"].as_array_mut().unwrap())
        .find(|outcome| {
            outcome["receipt"]["receipt"]["Action"]["actions"]
                .as_array()
                .is_some_and(|actions| {
                    actions
                        .iter()
                        .any(|action| action.get("Delegate").is_some())
                })
        })
        .unwrap();
    outcome["receipt"]["receiver_id"] = "unrelated.near".into();
    let delegate = outcome["receipt"]["receipt"]["Action"]["actions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find_map(|action| action.get_mut("Delegate"))
        .unwrap();
    delegate["delegate_action"]["actions"][0] =
        serde_json::json!({"FutureNestedAction": {"field": 1}});

    let block = InnerNearBlock::from_bytes(serde_json::to_vec(&source).unwrap()).unwrap();
    assert!(
        block
            .shards
            .iter()
            .flat_map(|shard| &shard.receipt_execution_outcomes)
            .any(
                |outcome| outcome.receipt.receiver_id.as_str() == "unrelated.near"
                    && matches!(outcome.receipt.receipt, ReceiptKind::Unsupported(_))
            )
    );
    block.validate_for_engine("aurora").unwrap();
}

#[test]
fn keeps_all_provenance_links_and_distinguishes_promise_results() {
    let mut source: near_indexer::StreamerMessage = serde_json::from_str(MAINNET_BLOCK).unwrap();
    let chunk = source
        .shards
        .iter_mut()
        .find_map(|shard| shard.chunk.as_mut())
        .unwrap();
    let make_receipt = |id, data| views::ReceiptView {
        predecessor_id: "caller.near".parse().unwrap(),
        receiver_id: "unrelated.near".parse().unwrap(),
        receipt_id: CryptoHash([id; 32]),
        receipt: views::ReceiptEnumView::Data {
            data_id: CryptoHash([id; 32]),
            data,
            is_promise_resume: false,
        },
        _priority: 0,
    };
    chunk.local_receipts = vec![make_receipt(1, None), make_receipt(2, Some(vec![]))];
    chunk.receipts = vec![make_receipt(3, Some(vec![42]))];
    let tx_links: Vec<_> = source
        .shards
        .iter()
        .filter_map(|shard| shard.chunk.as_ref())
        .flat_map(|chunk| &chunk.transactions)
        .map(|tx| {
            (
                tx.transaction.hash,
                tx.outcome.execution_outcome.outcome.receipt_ids.clone(),
            )
        })
        .collect();
    let receipt_links: Vec<_> = source
        .shards
        .iter()
        .flat_map(|shard| &shard.receipt_execution_outcomes)
        .map(|outcome| {
            (
                outcome.receipt.receipt_id,
                outcome.execution_outcome.outcome.receipt_ids.clone(),
            )
        })
        .collect();

    let block = InnerNearBlock::try_from(source).unwrap();
    let converted_tx_links: Vec<_> = block
        .shards
        .iter()
        .filter_map(|shard| shard.chunk.as_ref())
        .flat_map(|chunk| &chunk.transactions)
        .map(|tx| (tx.hash, tx.receipt_ids.clone()))
        .collect();
    let converted_receipt_links: Vec<_> = block
        .shards
        .iter()
        .flat_map(|shard| &shard.receipt_execution_outcomes)
        .map(|outcome| {
            (
                outcome.receipt.receipt_id,
                outcome.execution_outcome.receipt_ids.clone(),
            )
        })
        .collect();
    assert!(!tx_links.is_empty());
    assert!(!receipt_links.is_empty());
    assert_eq!(converted_tx_links, tx_links);
    assert_eq!(converted_receipt_links, receipt_links);
    let chunk = block
        .shards
        .iter()
        .find_map(|shard| shard.chunk.as_ref())
        .unwrap();
    let ordered: Vec<_> = chunk.local_receipts.iter().chain(&chunk.receipts).collect();
    assert_eq!(
        ordered.iter().map(|r| r.receipt_id).collect::<Vec<_>>(),
        vec![
            CryptoHash([1; 32]),
            CryptoHash([2; 32]),
            CryptoHash([3; 32])
        ]
    );
    assert_eq!(ordered[0].data.as_ref().unwrap().data, None);
    assert_eq!(ordered[1].data.as_ref().unwrap().data, Some(vec![]));
    assert_eq!(ordered[2].data.as_ref().unwrap().data, Some(vec![42]));
}

#[test]
fn preserves_storage_deletions_empty_values_and_legacy_causes_without_a_chunk() {
    let mut source = source_message();
    let shard = &mut source.shards[0];
    shard.chunk = None;
    let outcomes = shard.receipt_execution_outcomes.len();
    let receipt_hash = CryptoHash([4; 32]);
    shard.state_changes = vec![
        views::StateChangeWithCauseView {
            cause: views::StateChangeCauseView::ReceiptProcessing { receipt_hash },
            value: views::StateChangeValueView::DataUpdate {
                account_id: "aurora".parse().unwrap(),
                key: vec![1].into(),
                value: vec![].into(),
            },
        },
        views::StateChangeWithCauseView {
            cause: views::StateChangeCauseView::ReceiptProcessing { receipt_hash },
            value: views::StateChangeValueView::DataDeletion {
                account_id: "unrelated.near".parse().unwrap(),
                key: vec![2].into(),
            },
        },
        views::StateChangeWithCauseView {
            cause: views::StateChangeCauseView::Migration,
            value: views::StateChangeValueView::DataDeletion {
                account_id: "aurora".parse().unwrap(),
                key: vec![3].into(),
            },
        },
        views::StateChangeWithCauseView {
            cause: views::StateChangeCauseView::Migration,
            value: views::StateChangeValueView::AccountDeletion {
                account_id: "unrelated.near".parse().unwrap(),
            },
        },
    ];
    let mut json = serde_json::to_value(source).unwrap();
    json["shards"][0]["state_changes"][2]["cause"]["type"] = "resharding_v2".into();

    let json = serde_json::to_vec(&json).unwrap();
    let block = InnerNearBlock::from_bytes(&json).unwrap();
    let shard = &block.shards[0];
    assert!(shard.chunk.is_none());
    assert_eq!(shard.receipt_execution_outcomes.len(), outcomes);
    assert_eq!(shard.state_changes.len(), 3);
    assert_eq!(shard.state_changes[0].value, Some(vec![]));
    assert_eq!(shard.state_changes[1].value, None);
    assert_eq!(shard.state_changes[1].account_id.as_str(), "unrelated.near");
    assert_eq!(
        shard.state_changes[2].cause,
        StateChangeCause::Other("ReshardingV2".into())
    );
}

#[test]
fn ignores_new_fields_in_unneeded_chunk_receipts_and_preserves_unknown_causes() {
    let mut source: serde_json::Value = serde_json::from_str(HISTORICAL_BLOCK).unwrap();
    let shard_index = source["shards"]
        .as_array()
        .unwrap()
        .iter()
        .position(|shard| {
            shard["state_changes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|change| change["type"] == "data_update" || change["type"] == "data_deletion")
        })
        .unwrap();
    let shard = &mut source["shards"][shard_index];
    let change = shard["state_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|change| change["type"] == "data_update")
        .unwrap();
    change["cause"] = serde_json::json!({
        "type": "future_protocol_cause", "new_field": 42, "receipt_hash": "future-format"
    });
    let receipt = &mut shard["chunk"]["receipts"][0]["receipt"];
    *receipt = serde_json::json!({"FutureReceiptKind": {"new_field": 42}});
    let hash = source["block"]["header"]["hash"].clone();
    source["shards"][shard_index]["chunk"]["transactions"] = serde_json::json!([{
        "transaction": {"hash": hash},
        "outcome": {"execution_outcome": {"outcome": {
            "receipt_ids": [], "status": {"FutureExecutionStatus": {"new_field": 42}}
        }}}
    }]);

    let block = InnerNearBlock::from_bytes(serde_json::to_vec(&source).unwrap()).unwrap();
    assert!(
        block
            .shards
            .iter()
            .flat_map(|shard| &shard.state_changes)
            .any(|change| change.cause
                == StateChangeCause::Other(
                    "UnknownStateChangeCause(future_protocol_cause)".into()
                ))
    );
    assert!(
        block
            .shards
            .iter()
            .flat_map(|shard| shard.chunk.iter())
            .flat_map(|chunk| &chunk.receipts)
            .any(|receipt| receipt.data.is_none())
    );
    assert_eq!(
        block.shards[shard_index]
            .chunk
            .as_ref()
            .unwrap()
            .transactions
            .len(),
        1
    );
}

#[test]
fn decodes_future_failure_without_inventing_borsh_bytes() {
    let mut source: serde_json::Value = serde_json::from_str(HISTORICAL_BLOCK).unwrap();
    let outcomes = source["shards"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find_map(|shard| {
            shard["receipt_execution_outcomes"]
                .as_array_mut()
                .filter(|outcomes| !outcomes.is_empty())
        })
        .unwrap();
    outcomes[0]["execution_outcome"]["outcome"]["status"] =
        serde_json::json!({"Failure": {"FutureExecutionError": {"code": 17}}});

    let block = InnerNearBlock::from_bytes(serde_json::to_vec(&source).unwrap()).unwrap();
    assert!(matches!(
        &block
            .shards
            .iter()
            .flat_map(|shard| &shard.receipt_execution_outcomes)
            .next()
            .unwrap()
            .execution_outcome
            .status,
        ExecutionStatus::UnencodedFailure(error) if error.contains("FutureExecutionError")
    ));
    let restored: InnerNearBlock =
        serde_json::from_slice(&serde_json::to_vec(&block).unwrap()).unwrap();
    assert_eq!(restored, block);
}

#[test]
fn future_execution_status_is_opaque_only_for_other_accounts() {
    let mut source: serde_json::Value = serde_json::from_str(HISTORICAL_BLOCK).unwrap();
    let outcome = source["shards"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|shard| shard["receipt_execution_outcomes"].as_array_mut().unwrap())
        .next()
        .unwrap();
    outcome["receipt"]["receiver_id"] = "unrelated.near".into();
    outcome["execution_outcome"]["outcome"]["status"] =
        serde_json::json!({"FutureStatus": {"field": 1}});

    let block = InnerNearBlock::from_bytes(serde_json::to_vec(&source).unwrap()).unwrap();
    assert!(
        block
            .shards
            .iter()
            .flat_map(|shard| &shard.receipt_execution_outcomes)
            .any(|outcome| matches!(
                outcome.execution_outcome.status,
                ExecutionStatus::Unsupported(_)
            ))
    );
    block.validate_for_engine("aurora").unwrap();
    assert!(block.validate_for_engine("unrelated.near").is_err());
}

#[test]
fn keeps_provenance_for_unknown_external_receipt_without_inventing_borsh_size() {
    let mut source: serde_json::Value = serde_json::from_str(HISTORICAL_BLOCK).unwrap();
    let outcome = source["shards"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|shard| shard["receipt_execution_outcomes"].as_array_mut().unwrap())
        .find(|outcome| outcome["receipt"]["receipt"].get("Action").is_some())
        .unwrap();
    let receipt_id: CryptoHash = outcome["receipt"]["receipt_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    outcome["receipt"]["receipt"]["Action"]["actions"][0] =
        serde_json::json!({"FutureAction": {"unknown_field": true}});

    let block = InnerNearBlock::from_bytes(serde_json::to_vec(&source).unwrap()).unwrap();
    let outcome = block
        .shards
        .iter()
        .flat_map(|shard| &shard.receipt_execution_outcomes)
        .find(|outcome| matches!(outcome.receipt.receipt, ReceiptKind::Unsupported(_)))
        .unwrap();
    assert!(matches!(
        &outcome.receipt.receipt,
        ReceiptKind::Unsupported(reason) if reason.contains("unknown variant")
    ));
    assert_eq!(outcome.receipt_size, None);
    assert_eq!(outcome.receipt.receipt_id, receipt_id);
    let restored: InnerNearBlock =
        serde_json::from_slice(&serde_json::to_vec(&block).unwrap()).unwrap();
    assert_eq!(restored, block);
}

#[test]
fn rejects_unsupported_state_change_cause_only_for_engine_account() {
    let mut source = source_message();
    source.shards[0].state_changes = vec![views::StateChangeWithCauseView {
        cause: views::StateChangeCauseView::Migration,
        value: views::StateChangeValueView::DataUpdate {
            account_id: "aurora".parse().unwrap(),
            key: vec![1].into(),
            value: vec![2].into(),
        },
    }];

    let block = InnerNearBlock::try_from(source).unwrap();
    block.validate_for_engine("unrelated.near").unwrap();
    let error = block.validate_for_engine("aurora").unwrap_err().to_string();
    assert!(error.contains("unsupported cause of Aurora storage change: Migration"));
}

// Test block 151768255 from mainnet
#[test]
fn test_de_nearblock_151768255_mainnet_from_json() {
    InnerNearBlock::from_bytes(include_bytes!(
        "../../tests/res/near_block/block_151768255_mainnet.json"
    ))
    .expect("Failed to load InnerNearBlock");
}

#[test]
#[ignore]
fn test_de_nearblock_mainnet_latest_finalized_api_fetch() {
    check_latest_finalized_block("mainnet");
}

#[test]
#[ignore]
fn test_de_nearblock_testnet_latest_finalized_api_fetch() {
    check_latest_finalized_block("testnet");
}

fn check_latest_finalized_block(network: &str) {
    let client = reqwest::blocking::Client::new();
    let response_text = fetch_block(&client, network, "last_block/final");
    let height = extract_block_height(&response_text);
    println!("Latest finalized block height on {network}: {height}");
    assert_block_parses(&response_text, network, height);
}

// Sample blocks from 100_000_000 to the latest height to find unsupported ranges.
#[test]
#[ignore]
fn test_de_nearblock_both_networks_range_100m_to_latest_10m_step_api_fetch() {
    let client = reqwest::blocking::Client::new();
    println!();
    for network in ["mainnet", "testnet"] {
        println!("Testing {network} network...");
        let latest_height =
            extract_block_height(&fetch_block(&client, network, "last_block/final"));

        for height in (100_000_000..=latest_height).step_by(10_000_000) {
            println!("Test NEARBlock at height: {height} on {network}");
            let response_text = fetch_block(&client, network, &format!("block/{height}"));

            if response_text.trim() == "null" {
                println!("No block at height: {height} on {network}; skipping");
                continue;
            }

            assert_block_parses(&response_text, network, height);
        }
    }
}

fn fetch_block(client: &reqwest::blocking::Client, network: &str, path: &str) -> String {
    let url = format!("https://{network}.neardata.xyz/v0/{path}");
    client
        .get(&url)
        .send()
        .unwrap_or_else(|e| panic!("Failed to fetch {url}: {e}"))
        .error_for_status()
        .unwrap_or_else(|e| panic!("Unexpected response from {url}: {e}"))
        .text()
        .unwrap_or_else(|e| panic!("Failed to read response from {url}: {e}"))
}

fn assert_block_parses(response_text: &str, network: &str, height: u64) {
    InnerNearBlock::from_bytes(response_text.as_bytes()).unwrap_or_else(|e| {
        panic!("NEARBlock parse error: {e}, height: {height}, network: {network}")
    });
}

fn extract_block_height(response_text: &str) -> u64 {
    let json: serde_json::Value = serde_json::from_str(response_text).unwrap();
    json["block"]["header"]["height"].as_u64().unwrap()
}

#[test]
fn every_action_view_variant_matches_nearcore_borsh() {
    use near_crypto::{KeyType, PublicKey};

    let pk = || PublicKey::empty(KeyType::ED25519);
    let yocto = Balance::from_yoctonear;
    let variants = vec![
        views::ActionView::CreateAccount,
        views::ActionView::DeployContract {
            code: vec![1, 2, 3],
        },
        views::ActionView::Transfer { deposit: yocto(1) },
        views::ActionView::Stake {
            stake: yocto(2),
            public_key: pk(),
        },
        views::ActionView::AddKey {
            public_key: pk(),
            access_key: views::AccessKeyView {
                nonce: 1,
                permission: views::AccessKeyPermissionView::FullAccess,
            },
        },
        views::ActionView::AddKey {
            public_key: pk(),
            access_key: views::AccessKeyView {
                nonce: 2,
                permission: views::AccessKeyPermissionView::GasKeyFunctionCall {
                    balance: yocto(3),
                    num_nonces: 4,
                    allowance: Some(yocto(5)),
                    receiver_id: "aurora".into(),
                    method_names: vec!["submit".into()],
                },
            },
        },
        views::ActionView::AddKey {
            public_key: pk(),
            access_key: views::AccessKeyView {
                nonce: 3,
                permission: views::AccessKeyPermissionView::GasKeyFullAccess {
                    balance: yocto(6),
                    num_nonces: 7,
                },
            },
        },
        views::ActionView::DeleteKey { public_key: pk() },
        views::ActionView::DeleteAccount {
            beneficiary_id: "bob.near".parse().unwrap(),
        },
        views::ActionView::DeployGlobalContract { code: vec![4] },
        views::ActionView::DeployGlobalContractByAccountId { code: vec![5] },
        views::ActionView::UseGlobalContract {
            code_hash: CryptoHash([1; 32]),
        },
        views::ActionView::UseGlobalContractByAccountId {
            account_id: "code.near".parse().unwrap(),
        },
        views::ActionView::DeterministicStateInit {
            code: views::GlobalContractIdentifierView::AccountId("code.near".parse().unwrap()),
            data: [(vec![1], vec![2]), (vec![], vec![3])].into(),
            deposit: yocto(8),
        },
        views::ActionView::TransferToGasKey {
            public_key: pk(),
            deposit: yocto(9),
        },
        views::ActionView::WithdrawFromGasKey {
            public_key: pk(),
            amount: yocto(10),
        },
        // + Delegate / DelegateV2 built from a signed fixture
    ];

    let mut source = source_message();
    let outcome = source
        .shards
        .iter_mut()
        .flat_map(|shard| &mut shard.receipt_execution_outcomes)
        .find(|o| matches!(o.receipt.receipt, views::ReceiptEnumView::Action { .. }))
        .unwrap();
    let views::ReceiptEnumView::Action {
        actions, refund_to, ..
    } = &mut outcome.receipt.receipt
    else {
        unreachable!()
    };
    *actions = variants.clone();
    *refund_to = Some("refund.near".parse().unwrap()); // exercise the Option field too
    let id = outcome.receipt.receipt_id;
    let expected_size = borsh::object_length(&outcome.receipt).unwrap() as u64;

    let from_json = InnerNearBlock::from_bytes(serde_json::to_vec(&source).unwrap()).unwrap();
    assert_eq!(from_json, InnerNearBlock::try_from(source).unwrap());

    let outcome = from_json
        .shards
        .iter()
        .flat_map(|s| &s.receipt_execution_outcomes)
        .find(|o| o.receipt.receipt_id == id)
        .unwrap();
    assert_eq!(outcome.receipt_size, Some(expected_size));
    let ReceiptKind::Action { actions, .. } = &outcome.receipt.receipt else {
        panic!("expected action receipt")
    };
    for (view, action) in variants.iter().zip(actions) {
        assert_eq!(
            action,
            &Action::Other {
                borsh_bytes: borsh::to_vec(view).unwrap()
            },
            "{view:?}"
        );
    }
}
