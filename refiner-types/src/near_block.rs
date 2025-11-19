use borsh::BorshSerialize;
use near_crypto::{PublicKey, Signature};
use near_primitives::challenge::SlashedValidator;
use near_primitives::hash::CryptoHash;
use near_primitives::serialize::dec_format;
use near_primitives::types::{
    AccountId, Balance, BlockHeight, Gas, Nonce, NumBlocks, ProtocolVersion, ShardId, StateRoot,
};
use near_primitives::views;
use near_primitives::views::ActionView;
use near_primitives::views::validator_stake_view::ValidatorStakeView;
use serde::{Deserialize, Serialize};

/// Resulting struct represents block with chunks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NEARBlock {
    pub block: BlockView,
    pub shards: Vec<Shard>,
}

/// Backward-compatible version of StateChangeCauseView that includes removed variants
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StateChangeCauseView {
    NotWritableToDisk,
    InitialState,
    TransactionProcessing {
        tx_hash: CryptoHash,
    },
    ActionReceiptProcessingStarted {
        receipt_hash: CryptoHash,
    },
    ActionReceiptGasReward {
        receipt_hash: CryptoHash,
    },
    ReceiptProcessing {
        receipt_hash: CryptoHash,
    },
    PostponedReceipt {
        receipt_hash: CryptoHash,
    },
    UpdatedDelayedReceipts,
    ValidatorAccountsUpdate,
    Migration,
    /// Removed in nearcore 2.7.0-rc.1 but we keep it for backward compatibility
    ReshardingV2,
    BandwidthSchedulerStateUpdate,
}

/// Backward-compatible version of StateChangeWithCauseView
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateChangeWithCauseView {
    pub cause: StateChangeCauseView,
    #[serde(flatten)]
    pub value: views::StateChangeValueView,
}

/// Backward-compatible version of StateChangesView
pub type StateChangesView = Vec<StateChangeWithCauseView>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockView {
    pub author: AccountId,
    pub header: IndexerBlockHeaderView,
}

impl From<views::BlockView> for BlockView {
    fn from(view: views::BlockView) -> Self {
        Self {
            author: view.author,
            header: view.header.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexerBlockHeaderView {
    pub height: BlockHeight,
    pub prev_height: Option<BlockHeight>,
    pub epoch_id: CryptoHash,
    pub next_epoch_id: CryptoHash,
    pub hash: CryptoHash,
    pub prev_hash: CryptoHash,
    pub prev_state_root: CryptoHash,
    pub chunk_receipts_root: CryptoHash,
    pub chunk_headers_root: CryptoHash,
    pub chunk_tx_root: CryptoHash,
    pub outcome_root: CryptoHash,
    pub chunks_included: u64,
    pub challenges_root: CryptoHash,
    /// Legacy json number. Should not be used.
    pub timestamp: u64,
    #[serde(with = "dec_format")]
    pub timestamp_nanosec: u64,
    pub random_value: CryptoHash,
    pub validator_proposals: Vec<ValidatorStakeView>,
    pub chunk_mask: Vec<bool>,
    pub gas_price: Balance,
    pub block_ordinal: Option<NumBlocks>,
    pub total_supply: Balance,
    pub challenges_result: Vec<SlashedValidator>,
    pub last_final_block: CryptoHash,
    pub last_ds_final_block: CryptoHash,
    pub next_bp_hash: CryptoHash,
    pub block_merkle_root: CryptoHash,
    pub epoch_sync_data_hash: Option<CryptoHash>,
    pub approvals: Vec<Option<Box<Signature>>>,
    pub signature: Signature,
    pub latest_protocol_version: ProtocolVersion,
}

impl From<views::BlockHeaderView> for IndexerBlockHeaderView {
    fn from(header: views::BlockHeaderView) -> Self {
        let views::BlockHeaderView {
            height,
            prev_height,
            epoch_id,
            next_epoch_id,
            hash,
            prev_hash,
            prev_state_root,
            chunk_receipts_root,
            chunk_headers_root,
            chunk_tx_root,
            outcome_root,
            chunks_included,
            challenges_root,
            timestamp,
            timestamp_nanosec,
            random_value,
            validator_proposals,
            chunk_mask,
            gas_price,
            block_ordinal,
            total_supply,
            challenges_result,
            last_final_block,
            last_ds_final_block,
            next_bp_hash,
            block_merkle_root,
            epoch_sync_data_hash,
            approvals,
            signature,
            latest_protocol_version,
            ..
        } = header;
        Self {
            height,
            prev_height,
            epoch_id,
            next_epoch_id,
            hash,
            prev_hash,
            prev_state_root,
            chunk_receipts_root,
            chunk_headers_root,
            chunk_tx_root,
            outcome_root,
            chunks_included,
            challenges_root,
            timestamp,
            timestamp_nanosec,
            random_value,
            validator_proposals,
            chunk_mask,
            gas_price,
            block_ordinal,
            total_supply,
            challenges_result,
            last_final_block,
            last_ds_final_block,
            next_bp_hash,
            block_merkle_root,
            epoch_sync_data_hash,
            approvals,
            signature,
            latest_protocol_version,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkHeaderView {
    pub chunk_hash: CryptoHash,
    pub prev_block_hash: CryptoHash,
    pub outcome_root: CryptoHash,
    pub prev_state_root: StateRoot,
    pub encoded_merkle_root: CryptoHash,
    pub encoded_length: u64,
    pub height_created: BlockHeight,
    pub height_included: BlockHeight,
    pub shard_id: ShardId,
    pub gas_used: Gas,
    pub gas_limit: Gas,
    pub validator_reward: Balance,
    pub balance_burnt: Balance,
    pub outgoing_receipts_root: CryptoHash,
    pub tx_root: CryptoHash,
    pub validator_proposals: Vec<ValidatorStakeView>,
    pub signature: Signature,
}

impl From<views::ChunkHeaderView> for ChunkHeaderView {
    fn from(view: views::ChunkHeaderView) -> Self {
        let views::ChunkHeaderView {
            chunk_hash,
            prev_block_hash,
            outcome_root,
            prev_state_root,
            encoded_merkle_root,
            encoded_length,
            height_created,
            height_included,
            shard_id,
            gas_used,
            gas_limit,
            validator_reward,
            balance_burnt,
            outgoing_receipts_root,
            tx_root,
            validator_proposals,
            signature,
            ..
        } = view;
        Self {
            chunk_hash,
            prev_block_hash,
            outcome_root,
            prev_state_root,
            encoded_merkle_root,
            encoded_length,
            height_created,
            height_included,
            shard_id,
            gas_used,
            gas_limit,
            validator_reward,
            balance_burnt,
            outgoing_receipts_root,
            tx_root,
            validator_proposals,
            signature,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shard {
    pub shard_id: ShardId,
    pub chunk: Option<ChunkView>,
    pub receipt_execution_outcomes: Vec<ExecutionOutcomeWithReceipt>,
    pub state_changes: StateChangesView,
    // pub state_changes_views: views::StateChangesView,
}

impl Clone for Shard {
    fn clone(&self) -> Self {
        Self {
            shard_id: self.shard_id,
            chunk: self.chunk.clone(),
            receipt_execution_outcomes: self.receipt_execution_outcomes.clone(),
            state_changes: self
                .state_changes
                .iter()
                .map(|v: &StateChangeWithCauseView| StateChangeWithCauseView {
                    cause: match &v.cause {
                        StateChangeCauseView::NotWritableToDisk => {
                            StateChangeCauseView::NotWritableToDisk
                        }
                        StateChangeCauseView::InitialState => StateChangeCauseView::InitialState,
                        StateChangeCauseView::TransactionProcessing { tx_hash } => {
                            StateChangeCauseView::TransactionProcessing { tx_hash: *tx_hash }
                        }
                        StateChangeCauseView::ActionReceiptProcessingStarted { receipt_hash } => {
                            StateChangeCauseView::ActionReceiptProcessingStarted {
                                receipt_hash: *receipt_hash,
                            }
                        }
                        StateChangeCauseView::ActionReceiptGasReward { receipt_hash } => {
                            StateChangeCauseView::ActionReceiptGasReward {
                                receipt_hash: *receipt_hash,
                            }
                        }
                        StateChangeCauseView::ReceiptProcessing { receipt_hash } => {
                            StateChangeCauseView::ReceiptProcessing {
                                receipt_hash: *receipt_hash,
                            }
                        }
                        StateChangeCauseView::PostponedReceipt { receipt_hash } => {
                            StateChangeCauseView::PostponedReceipt {
                                receipt_hash: *receipt_hash,
                            }
                        }
                        StateChangeCauseView::UpdatedDelayedReceipts => {
                            StateChangeCauseView::UpdatedDelayedReceipts
                        }
                        StateChangeCauseView::ValidatorAccountsUpdate => {
                            StateChangeCauseView::ValidatorAccountsUpdate
                        }
                        StateChangeCauseView::Migration => StateChangeCauseView::Migration,
                        StateChangeCauseView::ReshardingV2 => StateChangeCauseView::ReshardingV2,
                        StateChangeCauseView::BandwidthSchedulerStateUpdate => {
                            StateChangeCauseView::BandwidthSchedulerStateUpdate
                        }
                    },
                    value: match &v.value {
                        views::StateChangeValueView::AccountUpdate {
                            account_id,
                            account,
                        } => views::StateChangeValueView::AccountUpdate {
                            account_id: account_id.clone(),
                            account: account.clone(),
                        },
                        views::StateChangeValueView::AccountDeletion { account_id } => {
                            views::StateChangeValueView::AccountDeletion {
                                account_id: account_id.clone(),
                            }
                        }
                        views::StateChangeValueView::AccessKeyUpdate {
                            account_id,
                            public_key,
                            access_key,
                        } => views::StateChangeValueView::AccessKeyUpdate {
                            account_id: account_id.clone(),
                            public_key: public_key.clone(),
                            access_key: access_key.clone(),
                        },
                        views::StateChangeValueView::AccessKeyDeletion {
                            account_id,
                            public_key,
                        } => views::StateChangeValueView::AccessKeyDeletion {
                            account_id: account_id.clone(),
                            public_key: public_key.clone(),
                        },
                        views::StateChangeValueView::GasKeyUpdate {
                            account_id,
                            public_key,
                            gas_key,
                        } => views::StateChangeValueView::GasKeyUpdate {
                            account_id: account_id.clone(),
                            public_key: public_key.clone(),
                            gas_key: gas_key.clone(),
                        },
                        views::StateChangeValueView::GasKeyNonceUpdate {
                            account_id,
                            public_key,
                            index,
                            nonce,
                        } => views::StateChangeValueView::GasKeyNonceUpdate {
                            account_id: account_id.clone(),
                            public_key: public_key.clone(),
                            index: *index,
                            nonce: *nonce,
                        },
                        views::StateChangeValueView::GasKeyDeletion {
                            account_id,
                            public_key,
                        } => views::StateChangeValueView::GasKeyDeletion {
                            account_id: account_id.clone(),
                            public_key: public_key.clone(),
                        },
                        views::StateChangeValueView::DataUpdate {
                            account_id,
                            key,
                            value,
                        } => views::StateChangeValueView::DataUpdate {
                            account_id: account_id.clone(),
                            key: key.clone(),
                            value: value.clone(),
                        },
                        views::StateChangeValueView::DataDeletion { account_id, key } => {
                            views::StateChangeValueView::DataDeletion {
                                account_id: account_id.clone(),
                                key: key.clone(),
                            }
                        }
                        views::StateChangeValueView::ContractCodeUpdate { account_id, code } => {
                            views::StateChangeValueView::ContractCodeUpdate {
                                account_id: account_id.clone(),
                                code: code.clone(),
                            }
                        }
                        views::StateChangeValueView::ContractCodeDeletion { account_id } => {
                            views::StateChangeValueView::ContractCodeDeletion {
                                account_id: account_id.clone(),
                            }
                        }
                    },
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkView {
    pub author: AccountId,
    pub header: ChunkHeaderView,
    pub transactions: Vec<TransactionWithOutcome>,
    pub receipts: Vec<ReceiptView>,
    #[serde(default)]
    pub local_receipts: Vec<ReceiptView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionWithOutcome {
    pub transaction: SignedTransactionView,
    pub outcome: ExecutionOutcomeWithOptionalReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionOutcomeWithOptionalReceipt {
    pub execution_outcome: views::ExecutionOutcomeWithIdView,
    pub receipt: Option<ReceiptView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionOutcomeWithReceipt {
    pub execution_outcome: views::ExecutionOutcomeWithIdView,
    pub receipt: ReceiptView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedTransactionView {
    pub signer_id: AccountId,
    pub public_key: PublicKey,
    pub nonce: Nonce,
    pub receiver_id: AccountId,
    pub actions: Vec<ActionView>,
    #[serde(default)]
    pub priority_fee: u64,
    pub signature: Signature,
    pub hash: CryptoHash,
}

impl From<views::SignedTransactionView> for SignedTransactionView {
    fn from(value: views::SignedTransactionView) -> Self {
        Self {
            signer_id: value.signer_id,
            public_key: value.public_key,
            nonce: value.nonce,
            receiver_id: value.receiver_id,
            actions: value.actions.clone(),
            priority_fee: value.priority_fee,
            signature: value.signature,
            hash: value.hash,
        }
    }
}

#[derive(Clone, Debug, BorshSerialize, Serialize, Deserialize)]
pub struct ReceiptView {
    pub predecessor_id: AccountId,
    pub receiver_id: AccountId,
    pub receipt_id: CryptoHash,
    pub receipt: views::ReceiptEnumView,
    #[serde(default)]
    pub priority: u64,
}

impl From<views::ReceiptView> for ReceiptView {
    fn from(value: views::ReceiptView) -> Self {
        Self {
            predecessor_id: value.predecessor_id,
            receiver_id: value.receiver_id,
            receipt_id: value.receipt_id,
            receipt: value.receipt.clone(),
            priority: value.priority,
        }
    }
}

// Conversion functions for backward-compatible types
impl StateChangeCauseView {
    pub const fn from_nearcore(cause: views::StateChangeCauseView) -> Self {
        match cause {
            views::StateChangeCauseView::NotWritableToDisk => Self::NotWritableToDisk,
            views::StateChangeCauseView::InitialState => Self::InitialState,
            views::StateChangeCauseView::TransactionProcessing { tx_hash } => {
                Self::TransactionProcessing { tx_hash }
            }
            views::StateChangeCauseView::ActionReceiptProcessingStarted { receipt_hash } => {
                Self::ActionReceiptProcessingStarted { receipt_hash }
            }
            views::StateChangeCauseView::ActionReceiptGasReward { receipt_hash } => {
                Self::ActionReceiptGasReward { receipt_hash }
            }
            views::StateChangeCauseView::ReceiptProcessing { receipt_hash } => {
                Self::ReceiptProcessing { receipt_hash }
            }
            views::StateChangeCauseView::PostponedReceipt { receipt_hash } => {
                Self::PostponedReceipt { receipt_hash }
            }
            views::StateChangeCauseView::UpdatedDelayedReceipts => Self::UpdatedDelayedReceipts,
            views::StateChangeCauseView::ValidatorAccountsUpdate => Self::ValidatorAccountsUpdate,
            views::StateChangeCauseView::Migration => Self::Migration,
            views::StateChangeCauseView::BandwidthSchedulerStateUpdate => {
                Self::BandwidthSchedulerStateUpdate
            }
        }
    }
}

impl StateChangeWithCauseView {
    pub fn from_nearcore(change: views::StateChangeWithCauseView) -> Self {
        Self {
            cause: StateChangeCauseView::from_nearcore(change.cause),
            value: change.value,
        }
    }
}

pub fn convert_state_changes(changes: views::StateChangesView) -> StateChangesView {
    changes
        .into_iter()
        .map(StateChangeWithCauseView::from_nearcore)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test block 151768255 from mainnet
    #[test]
    fn test_de_nearblock_151768255_mainnet_from_json() {
        serde_json::from_str::<NEARBlock>(include_str!(
            "../tests/res/near_block/block_151768255_mainnet.json"
        ))
        .expect("Failed to load NEARBlock");
    }

    // Test latest block from mainnet using request GET https://mainnet.neardata.xyz/v0/last_block/final
    #[test]
    #[ignore]
    fn test_de_nearblock_mainnet_latest_finalized_api_fetch() {
        let response = reqwest::blocking::Client::new()
            .get("https://mainnet.neardata.xyz/v0/last_block/final")
            .send()
            .expect("Failed to fetch latest block from mainnet");

        let response_text = response.text().expect("Failed to get response text");
        let latest_height = extract_block_height(&response_text);
        println!("Latest finalized block height on mainnet: {latest_height}");

        serde_json::from_str::<NEARBlock>(&response_text)
            .inspect_err(|e| {
                let height = extract_block_height(&response_text);
                println!("NEARBlock parse error: {e}, height: {height}");
            })
            .expect("Failed to parse block");
    }

    // Test latest block from testnet using request GET https://testnet.neardata.xyz/v0/last_block/final
    #[test]
    #[ignore]
    fn test_de_nearblock_testnet_latest_finalized_api_fetch() {
        let response = reqwest::blocking::Client::new()
            .get("https://testnet.neardata.xyz/v0/last_block/final")
            .send()
            .expect("Failed to fetch latest block from testnet");

        let response_text = response.text().expect("Failed to get response text");
        let latest_height = extract_block_height(&response_text);
        println!("Latest finalized block height on testnet: {latest_height}");

        serde_json::from_str::<NEARBlock>(&response_text)
            .inspect_err(|e| {
                let height = extract_block_height(&response_text);
                println!("NEARBlock parse error: {e}, height: {height}");
            })
            .expect("Failed to parse block");
    }

    // Test block range from 100_000_000 to LATEST on both mainnet and testnet with step 10_000_000
    // The purpose of this test is to identify the block range that is not supported by the NEARBlock deserialization
    #[test]
    #[ignore]
    fn test_de_nearblock_both_networks_range_100m_to_latest_10m_step_api_fetch() {
        let networks = [
            ("mainnet", "https://mainnet.neardata.xyz/v0"),
            ("testnet", "https://testnet.neardata.xyz/v0"),
        ];

        for (network_name, base_url) in networks {
            println!("Testing {network_name} network...");

            // Get latest block height
            let response = reqwest::blocking::Client::new()
                .get(format!("{base_url}/last_block/final"))
                .send()
                .unwrap_or_else(|_| panic!("Failed to fetch latest block from {network_name}"));
            let response_text = response.text().expect("Failed to get response text");
            let latest_height = extract_block_height(&response_text);

            for height in (100_000_000..=latest_height).step_by(10_000_000) {
                println!("Test NEARBlock at height: {height} on {network_name}");

                let response = reqwest::blocking::Client::new()
                    .get(format!("{base_url}/block/{height}"))
                    .send()
                    .unwrap_or_else(|_| panic!("Failed to fetch block from {network_name}"));

                let response_text = response.text().expect("Failed to get response text");

                serde_json::from_str::<NEARBlock>(&response_text)
                    .inspect_err(|e| {
                        let height = extract_block_height(&response_text);
                        println!(
                            "NEARBlock parse error: {e}, height: {height}, network: {network_name}"
                        );
                    })
                    .unwrap_or_else(|_| panic!("Failed to parse block on {network_name}"));
            }
        }
    }

    // Helper function to extract block height from JSON response text
    fn extract_block_height(response_text: &str) -> u64 {
        let json: serde_json::Value = serde_json::from_str(response_text).unwrap();
        json["block"]["header"]["height"].as_u64().unwrap()
    }
}
