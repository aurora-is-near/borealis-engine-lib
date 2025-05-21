use borsh::BorshSerialize;
use near_crypto::{PublicKey, Signature};
use near_primitives::challenge::ChallengesResult;
use near_primitives::hash::CryptoHash;
use near_primitives::serialize::dec_format;
use near_primitives::types::{
    AccountId, Balance, BlockHeight, Gas, Nonce, NumBlocks, ProtocolVersion, ShardId, StateRoot,
};
use near_primitives::views;
use near_primitives::views::validator_stake_view::ValidatorStakeView;
use near_primitives::views::{
    ActionView, StateChangeCauseView, StateChangeValueView, StateChangeWithCauseView,
};
use serde::{Deserialize, Serialize};

/// Resulting struct represents block with chunks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NEARBlock {
    pub block: BlockView,
    pub shards: Vec<Shard>,
}

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
    #[serde(with = "dec_format")]
    pub gas_price: Balance,
    pub block_ordinal: Option<NumBlocks>,
    #[serde(with = "dec_format")]
    pub total_supply: Balance,
    pub challenges_result: ChallengesResult,
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
    #[serde(with = "dec_format")]
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
    pub state_changes: views::StateChangesView,
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
                        StateChangeValueView::AccountUpdate {
                            account_id,
                            account,
                        } => StateChangeValueView::AccountUpdate {
                            account_id: account_id.clone(),
                            account: account.clone(),
                        },
                        StateChangeValueView::AccountDeletion { account_id } => {
                            StateChangeValueView::AccountDeletion {
                                account_id: account_id.clone(),
                            }
                        }
                        StateChangeValueView::AccessKeyUpdate {
                            account_id,
                            public_key,
                            access_key,
                        } => StateChangeValueView::AccessKeyUpdate {
                            account_id: account_id.clone(),
                            public_key: public_key.clone(),
                            access_key: access_key.clone(),
                        },
                        StateChangeValueView::AccessKeyDeletion {
                            account_id,
                            public_key,
                        } => StateChangeValueView::AccessKeyDeletion {
                            account_id: account_id.clone(),
                            public_key: public_key.clone(),
                        },
                        StateChangeValueView::DataUpdate {
                            account_id,
                            key,
                            value,
                        } => StateChangeValueView::DataUpdate {
                            account_id: account_id.clone(),
                            key: key.clone(),
                            value: value.clone(),
                        },
                        StateChangeValueView::DataDeletion { account_id, key } => {
                            StateChangeValueView::DataDeletion {
                                account_id: account_id.clone(),
                                key: key.clone(),
                            }
                        }
                        StateChangeValueView::ContractCodeUpdate { account_id, code } => {
                            StateChangeValueView::ContractCodeUpdate {
                                account_id: account_id.clone(),
                                code: code.clone(),
                            }
                        }
                        StateChangeValueView::ContractCodeDeletion { account_id } => {
                            StateChangeValueView::ContractCodeDeletion {
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

impl From<near_lake_framework::near_indexer_primitives::views::BlockView> for BlockView {
    fn from(view: near_lake_framework::near_indexer_primitives::views::BlockView) -> Self {
        Self {
            author: view.author,
            header: view.header.into(),
        }
    }
}

impl From<near_lake_framework::near_indexer_primitives::views::BlockHeaderView>
    for IndexerBlockHeaderView
{
    fn from(header: near_lake_framework::near_indexer_primitives::views::BlockHeaderView) -> Self {
        let near_lake_framework::near_indexer_primitives::views::BlockHeaderView {
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
            epoch_id: CryptoHash(epoch_id.0),
            next_epoch_id: CryptoHash(next_epoch_id.0),
            hash: CryptoHash(hash.0),
            prev_hash: CryptoHash(prev_hash.0),
            prev_state_root: CryptoHash(prev_state_root.0),
            chunk_receipts_root: CryptoHash(chunk_receipts_root.0),
            chunk_headers_root: CryptoHash(chunk_headers_root.0),
            chunk_tx_root: CryptoHash(chunk_tx_root.0),
            outcome_root: CryptoHash(outcome_root.0),
            chunks_included,
            challenges_root: CryptoHash(challenges_root.0),
            timestamp,
            timestamp_nanosec,
            random_value: CryptoHash(random_value.0),
            validator_proposals: validator_proposals
                .into_iter()
                .map(|v| convert_validator_stake_view(v))
                .collect(),
            chunk_mask,
            gas_price,
            block_ordinal,
            total_supply,
            challenges_result: challenges_result.into_iter().map(|v| convert_slashed_validator(v)).collect(),
            last_final_block: CryptoHash(last_final_block.0),
            last_ds_final_block: CryptoHash(last_ds_final_block.0),
            next_bp_hash: CryptoHash(next_bp_hash.0),
            block_merkle_root: CryptoHash(block_merkle_root.0),
            epoch_sync_data_hash: epoch_sync_data_hash.map(|h| CryptoHash(h.0)),
            approvals: approvals
                .into_iter()
                .map(|v| v.map(|s| Box::new(convert_signature(*s)))).collect(),
            signature: convert_signature(signature),
            latest_protocol_version,
        }
    }
}

fn convert_validator_stake_view(
    v: near_lake_framework::near_indexer_primitives::views::validator_stake_view::ValidatorStakeView,
) -> near_primitives::views::validator_stake_view::ValidatorStakeView {
    match v {
        near_lake_framework::near_indexer_primitives::views::validator_stake_view::ValidatorStakeView::V1(inner) => {
            near_primitives::views::validator_stake_view::ValidatorStakeView::V1(
                near_primitives::views::validator_stake_view::ValidatorStakeViewV1 {
                    account_id: inner.account_id,
                    public_key: convert_public_key(inner.public_key),
                    stake: inner.stake,
                }
            )
        }
    }
}

pub fn convert_public_key(v: refiner_types_crates_io::PublicKeyCratesIo) -> near_crypto::PublicKey {
    let key_data = v.key_data();
    match v {
        refiner_types_crates_io::PublicKeyCratesIo::ED25519(inner) => {
            near_crypto::PublicKey::ED25519(inner.0.into())
        }
        refiner_types_crates_io::PublicKeyCratesIo::SECP256K1(_) => {
            near_crypto::PublicKey::SECP256K1(
                near_crypto::Secp256K1PublicKey::try_from(key_data).unwrap(), // TODO: Remove unwrap
            )
        }
    }
}

fn convert_slashed_validator(
    v: near_primitives_crates_io::challenge::SlashedValidator,
) -> near_primitives::challenge::SlashedValidator {
    near_primitives::challenge::SlashedValidator {
        account_id: v.account_id,
        is_double_sign: v.is_double_sign,
    }
}

pub fn convert_signature(v: refiner_types_crates_io::SignatureCratesIo) -> near_crypto::Signature {
    match v {
        refiner_types_crates_io::SignatureCratesIo::ED25519(inner) => {
            near_crypto::Signature::ED25519(inner)
        }
        refiner_types_crates_io::SignatureCratesIo::SECP256K1(inner) => {
            let r: [u8; 65] = inner.into();
            near_crypto::Signature::SECP256K1(near_crypto::Secp256K1Signature::from(r))
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
