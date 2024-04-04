use near_crypto::{Secp256K1Signature, Signature};
use near_primitives::challenge::ChallengesResult;
use near_primitives::hash::CryptoHash;
use near_primitives::serialize::dec_format;
use near_primitives::types::{
    AccountId, Balance, BlockHeight, Gas, NumBlocks, ProtocolVersion, ShardId, StateRoot,
};
use near_primitives::views;
use near_primitives::views::validator_stake_view::ValidatorStakeView;
use near_primitives::views::{StateChangeCauseView, StateChangeValueView, StateChangeWithCauseView, ValidatorStakeViewV1};
use serde::{Deserialize, Serialize};

use near_lake_framework::near_indexer_primitives;

/// Resulting struct represents block with chunks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NEARBlock {
    pub block: BlockView,
    pub shards: Vec<Shard>,
}

impl From<near_indexer_primitives::StreamerMessage> for NEARBlock {
    fn from(block: near_indexer_primitives::StreamerMessage) -> Self {
        Self {
            block: BlockView::from(block.block),
            shards: block.shards.into_iter().map(Shard::from).collect(),
        }
    }
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

impl From<near_indexer_primitives::views::BlockView> for BlockView {
    fn from(view: near_indexer_primitives::views::BlockView) -> Self {
        Self {
            author: view.author,
            header: view.header,
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

impl From<near_indexer_primitives::views::BlockHeaderView> for IndexerBlockHeaderView {
    fn from(header: near_indexer_primitives::views::BlockHeaderView) -> Self {
        Self {
            height: header.height,
            prev_height: header.prev_height,
            epoch_id: header.epoch_id.into_borealis_types(),
            next_epoch_id: header.next_epoch_id.into_borealis_types(),
            hash: header.hash.into_borealis_types(),
            prev_hash: header.prev_hash.into_borealis_types(),
            prev_state_root: header.prev_state_root.into_borealis_types(),
            chunk_receipts_root: header.chunk_receipts_root.into_borealis_types(),
            chunk_headers_root: header.chunk_headers_root.into_borealis_types(),
            chunk_tx_root: header.chunk_tx_root.into_borealis_types(),
            outcome_root: header.outcome_root.into_borealis_types(),
            chunks_included: header.chunks_included,
            challenges_root: header.challenges_root.into_borealis_types(),
            timestamp: header.timestamp,
            timestamp_nanosec: header.timestamp_nanosec,
            random_value: header.random_value.into_borealis_types(),
            validator_proposals: header.validator_proposals,
            chunk_mask: header.chunk_mask,
            gas_price: header.gas_price,
            block_ordinal: header.block_ordinal,
            total_supply: header.total_supply,
            challenges_result: header.challenges_result,
            last_final_block: header.last_final_block.into_borealis_types(),
            last_ds_final_block: header.last_ds_final_block.into_borealis_types(),
            next_bp_hash: header.next_bp_hash.into_borealis_types(),
            block_merkle_root: header.block_merkle_root.into_borealis_types(),
            epoch_sync_data_hash: header.epoch_sync_data_hash.map(|v| v.into_borealis_types()),
            approvals: header.approvals.into_iter().map(|v| v.map(|v| v.into_borealis_types())).collect(),
            signature: header.signature,
            latest_protocol_version: header.latest_protocol_version,
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
                        StateChangeCauseView::Resharding => StateChangeCauseView::Resharding,
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

impl From<near_indexer_primitives::IndexerShard> for Shard {
    fn from(shard: near_indexer_primitives::IndexerShard) -> Self {
        Self {
            shard_id: shard.shard_id,
            chunk: shard.chunk.map(ChunkView::from),
            receipt_execution_outcomes: shard
                .receipt_execution_outcomes
                .into_iter()
                .map(ExecutionOutcomeWithReceipt::from)
                .collect(),
            state_changes: shard.state_changes,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkView {
    pub author: AccountId,
    pub header: ChunkHeaderView,
    pub transactions: Vec<TransactionWithOutcome>,
    pub receipts: Vec<views::ReceiptView>,
}

impl From<near_indexer_primitives::IndexerChunkView> for ChunkView {
    fn from(chunk: near_indexer_primitives::IndexerChunkView) -> Self {
        Self {
            author: chunk.author,
            header: chunk.header.into(),
            transactions: chunk.transactions.into_iter().map(TransactionWithOutcome::from).collect(),
            receipts: chunk.receipts.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionWithOutcome {
    pub transaction: views::SignedTransactionView,
    pub outcome: ExecutionOutcomeWithOptionalReceipt,
}

impl From<near_indexer_primitives::IndexerTransactionWithOutcome> for TransactionWithOutcome {
    fn from(transaction: near_indexer_primitives::IndexerTransactionWithOutcome) -> Self {
        Self {
            transaction: transaction.transaction.into(),
            outcome: transaction.outcome.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionOutcomeWithOptionalReceipt {
    pub execution_outcome: views::ExecutionOutcomeWithIdView,
    pub receipt: Option<views::ReceiptView>,
}

impl From<near_indexer_primitives::IndexerExecutionOutcomeWithOptionalReceipt> for ExecutionOutcomeWithOptionalReceipt {
    fn from(outcome: near_indexer_primitives::IndexerExecutionOutcomeWithOptionalReceipt) -> Self {
        Self {
            execution_outcome: outcome.execution_outcome.into(),
            receipt: outcome.receipt.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionOutcomeWithReceipt {
    pub execution_outcome: views::ExecutionOutcomeWithIdView,
    pub receipt: views::ReceiptView,
}

trait IntoBorealisTypes<T> {
    fn into_borealis_types(self) -> T;
}

impl IntoBorealisTypes<CryptoHash> for near_indexer_primitives::CryptoHash {
    fn into_borealis_types(self) -> CryptoHash {
        CryptoHash(self.0)
    }
}

impl IntoBorealisTypes<Signature> for near_crypto::signature::Signature {
    fn into_borealis_types(self) -> Signature {
        match self {
            near_crypto::signature::Signature::ED25519(signature) => Signature::ED25519(signature),
            near_crypto::signature::Signature::SECP256K1(signature) => Signature::SECP256K1(signature),
        }
    }
}

impl IntoBorealisTypes<ValidatorStakeView> for near_indexer_primitives::views::validator_stake_view::ValidatorStakeView {
    fn into_borealis_types(self) -> ValidatorStakeView {
        match self {
            near_indexer_primitives::views::validator_stake_view::ValidatorStakeView::V1(validator_stake_view_v1) => {
                ValidatorStakeView::V1(validator_stake_view_v1.into_borealis_types())
            }
        }
    }
}