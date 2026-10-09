use super::*;
use block_client_rs::stream::read::ReadStream;
use block_client_rs::types::request::{BlocksRequestBuilder, DeliverySettings, StartPolicy};
use block_client_rs::types::{BlockMessage, BlockPayloadFormat};
use block_client_rs::{BlockClient, Config};
use near_primitives::{types::Balance, views};
use std::time::Duration;

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

#[tokio::test]
#[ignore = "requires BLOCK_CLIENT_MAINNET_URL and BLOCK_CLIENT_MAINNET_TOKEN"]
async fn test_de_nearblock_mainnet_latest_block_client_fetch() {
    check_latest_block("mainnet").await;
}

#[tokio::test]
#[ignore = "requires BLOCK_CLIENT_TESTNET_URL and BLOCK_CLIENT_TESTNET_TOKEN"]
async fn test_de_nearblock_testnet_latest_block_client_fetch() {
    check_latest_block("testnet").await;
}

async fn check_latest_block(network: &str) {
    let mut client = block_client(network);
    let (height, bytes) =
        fetch_block(&mut client, network, StartPolicy::StartOnLatestAvailable).await;
    println!("Latest available block height on {network}: {height}");
    assert_block_parses(&bytes, network, height);
}

// Sample the boundaries of 15 equal intervals across the available block range.
#[tokio::test]
#[ignore = "requires BLOCK_CLIENT_{MAINNET,TESTNET}_{URL,TOKEN}"]
async fn test_de_nearblock_both_networks_available_range_15_intervals_block_client_fetch() {
    for network in ["mainnet", "testnet"] {
        let mut client = block_client(network);

        let start_policy = match network {
            "mainnet" => StartPolicy::StartOnEarliestAvailable,
            "testnet" => StartPolicy::StartExactlyOnTarget(150_000_000),
            _ => panic!("Unsupported network: {network}"),
        };

        println!("Testing {network} network...");
        let (first_height, first_block) = fetch_block(&mut client, network, start_policy).await;
        let (latest_height, latest_block) =
            fetch_block(&mut client, network, StartPolicy::StartOnLatestAvailable).await;
        println!("Available block range on {network}: {first_height}..={latest_height}");
        assert_block_parses(&first_block, network, first_height);
        assert_block_parses(&latest_block, network, latest_height);

        for height in sample_block_heights(first_height, latest_height) {
            if height == first_height || height == latest_height {
                continue;
            }

            let (actual_height, bytes) = fetch_block(
                &mut client,
                network,
                StartPolicy::StartOnClosestToTarget(height),
            )
            .await;
            assert!(
                (height..=latest_height).contains(&actual_height),
                "Block {actual_height} is outside {height}..={latest_height} on {network}"
            );
            println!("Test NEARBlock at height: {actual_height} (target: {height}) on {network}");

            assert_block_parses(&bytes, network, actual_height);
        }
    }
}

fn block_client(network: &str) -> BlockClient {
    let prefix = format!("BLOCK_CLIENT_{}", network.to_ascii_uppercase());
    let config = Config {
        url: std::env::var(format!("{prefix}_URL"))
            .unwrap_or_else(|_| panic!("Set {prefix}_URL to the block service URL")),
        token: std::env::var(format!("{prefix}_TOKEN"))
            .unwrap_or_else(|_| panic!("Set {prefix}_TOKEN to the block service token")),
        stream_name: format!("v2_{network}_near_blocks"),
        connection_window_size: 64 * 1024 * 1024,
        stream_window_size: 64 * 1024 * 1024,
        request_timeout: 30,
        connect_timeout: 10,
        buffer_size: 256,
        max_message_size: 1024 * 1024 * 1024,
    };
    BlockClient::new(config)
        .unwrap_or_else(|e| panic!("Failed to create block client for {network}: {e}"))
}

async fn fetch_block(
    client: &mut BlockClient,
    network: &str,
    start_policy: StartPolicy,
) -> (u64, Vec<u8>) {
    let target = format!("{start_policy:?}");
    let request = BlocksRequestBuilder::new()
        .with_stream_name(format!("v2_{network}_near_blocks"))
        .with_start_policy(start_policy)
        .with_delivery_settings(DeliverySettings {
            exclude_payload: false,
            // The client does not decode transport compression. V2 payloads already use LZ4.
            allow_compression: 0,
        })
        .build();
    let message = tokio::time::timeout(Duration::from_secs(30), async {
        let mut blocks = client
            .get_block_stream(request)
            .await
            .unwrap_or_else(|e| panic!("Failed to open {network} block stream ({target}): {e}"));
        blocks
            .next()
            .await
            .unwrap_or_else(|e| panic!("Failed to fetch {network} block ({target}): {e}"))
    })
    .await
    .unwrap_or_else(|e| panic!("Timed out fetching {network} block ({target}): {e}"));
    let bytes = decode_block_payload(&message)
        .unwrap_or_else(|e| panic!("Failed to decode {network} block {}: {e}", message.height));
    assert_eq!(extract_block_height(&bytes), message.height);
    (message.height, bytes)
}

fn decode_block_payload(message: &BlockMessage) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if !matches!(message.format, BlockPayloadFormat::NearBlockV2) {
        return Err(format!("Unsupported block payload format: {:?}", message.format).into());
    }
    let (version, body) = message.payload.split_first().ok_or("Missing bus message")?;
    if *version != block_client_rs::types::bus_message::VERSION {
        return Err(format!("Unsupported bus message version: {version}").into());
    }

    // Decode the envelope and LZ4 payload without deserializing into the client's block types.
    let mut values = serde_cbor::Deserializer::from_slice(body).into_iter::<serde_cbor::Value>();
    let _envelope = values.next().ok_or("Missing bus envelope")??;
    let serde_cbor::Value::Bytes(compressed) = values.next().ok_or("Missing block payload")??
    else {
        return Err("Expected a CBOR byte string for the block payload".into());
    };
    if values.next().is_some() {
        return Err("Unexpected data after the block payload".into());
    }

    let mut decoder = lz4::Decoder::new(compressed.as_slice())?;
    let mut bytes = Vec::new();
    std::io::copy(&mut decoder, &mut bytes)?;
    decoder.finish().1?;
    Ok(bytes)
}

fn sample_block_heights(first: u64, last: u64) -> Vec<u64> {
    assert!(
        first <= last,
        "Invalid available block range: {first}..={last}"
    );
    const INTERVALS: u128 = 15;
    let range = u128::from(last - first);
    let mut heights = (0..=INTERVALS)
        .map(|i| first + (range * i / INTERVALS) as u64)
        .collect::<Vec<_>>();
    heights.dedup();
    heights
}

fn assert_block_parses(bytes: &[u8], network: &str, height: u64) {
    InnerNearBlock::from_bytes(bytes).unwrap_or_else(|e| {
        panic!("NEARBlock parse error: {e}, height: {height}, network: {network}")
    });
}

fn extract_block_height(bytes: &[u8]) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    json["block"]["header"]["height"].as_u64().unwrap()
}

#[test]
fn block_range_sampling_covers_endpoints_and_handles_short_and_large_ranges() {
    assert_eq!(sample_block_heights(100, 100), vec![100]);
    assert_eq!(sample_block_heights(100, 103), vec![100, 101, 102, 103]);
    for (first, last) in [(100, 132), (0, u64::MAX)] {
        let heights = sample_block_heights(first, last);
        assert_eq!(heights.len(), 16);
        assert_eq!(heights.first(), Some(&first));
        assert_eq!(heights.last(), Some(&last));
        let step = (last - first) / 15;
        assert!(heights.windows(2).all(|pair| {
            let gap = pair[1] - pair[0];
            gap == step || gap == step + 1
        }));
    }
}

#[test]
fn block_client_payload_preserves_raw_json_with_unknown_fields() {
    let bytes = br#"{"block":{"header":{"height":42}},"future_field":{"FutureAction":{}}}"#;
    let mut encoder = lz4::EncoderBuilder::new().build(Vec::new()).unwrap();
    std::io::copy(&mut bytes.as_slice(), &mut encoder).unwrap();
    let (compressed, result) = encoder.finish();
    result.unwrap();

    let envelope = serde_cbor::Value::Array(vec![
        0x1030.into(),
        42.into(),
        0.into(),
        0.into(),
        serde_cbor::Value::Bytes(vec![0; 16]),
    ]);
    let mut payload = vec![block_client_rs::types::bus_message::VERSION];
    payload.extend(serde_cbor::to_vec(&envelope).unwrap());
    payload.extend(serde_cbor::to_vec(&serde_cbor::Value::Bytes(compressed)).unwrap());
    let message = BlockMessage {
        height: 42,
        payload,
        format: BlockPayloadFormat::NearBlockV2,
    };
    assert_eq!(decode_block_payload(&message).unwrap(), bytes);
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
