use near_crypto::Signature;
use near_primitives::challenge::ChallengesResult;
use near_primitives::hash::CryptoHash;
use near_primitives::serialize::{u128_dec_format, u64_dec_format};
use near_primitives::types::{
    AccountId, Balance, BlockHeight, Gas, NumBlocks, ProtocolVersion, ShardId, StateRoot,
};
use near_primitives::views::validator_stake_view::ValidatorStakeView;
use near_primitives::{types, views};
use serde::{Deserialize, Serialize};

/// Resulting struct represents block with chunks
#[derive(Debug, Serialize, Deserialize)]
pub struct NEARBlock {
    pub block: BlockView,
    pub shards: Vec<Shard>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockView {
    pub author: AccountId,
    pub header: IndexerBlockHeaderView,
}

impl From<near_primitives::views::BlockView> for BlockView {
    fn from(view: near_primitives::views::BlockView) -> Self {
        BlockView {
            author: view.author,
            header: view.header.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
    #[serde(with = "u64_dec_format")]
    pub timestamp_nanosec: u64,
    pub random_value: CryptoHash,
    pub validator_proposals: Vec<ValidatorStakeView>,
    pub chunk_mask: Vec<bool>,
    #[serde(with = "u128_dec_format")]
    pub gas_price: Balance,
    pub block_ordinal: Option<NumBlocks>,
    #[serde(with = "u128_dec_format")]
    pub total_supply: Balance,
    pub challenges_result: ChallengesResult,
    pub last_final_block: CryptoHash,
    pub last_ds_final_block: CryptoHash,
    pub next_bp_hash: CryptoHash,
    pub block_merkle_root: CryptoHash,
    pub epoch_sync_data_hash: Option<CryptoHash>,
    pub approvals: Vec<Option<Signature>>,
    pub signature: Signature,
    pub latest_protocol_version: ProtocolVersion,
}

impl From<near_primitives::views::BlockHeaderView> for IndexerBlockHeaderView {
    fn from(header: near_primitives::views::BlockHeaderView) -> Self {
        let near_primitives::views::BlockHeaderView {
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
        IndexerBlockHeaderView {
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
    #[serde(with = "u128_dec_format")]
    pub balance_burnt: Balance,
    pub outgoing_receipts_root: CryptoHash,
    pub tx_root: CryptoHash,
    pub validator_proposals: Vec<ValidatorStakeView>,
    pub signature: Signature,
}

impl From<near_primitives::views::ChunkHeaderView> for ChunkHeaderView {
    fn from(view: near_primitives::views::ChunkHeaderView) -> Self {
        let near_primitives::views::ChunkHeaderView {
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
        ChunkHeaderView {
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
    pub shard_id: types::ShardId,
    pub chunk: Option<ChunkView>,
    pub receipt_execution_outcomes: Vec<ExecutionOutcomeWithReceipt>,
    pub state_changes: views::StateChangesView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkView {
    pub author: types::AccountId,
    pub header: ChunkHeaderView,
    pub transactions: Vec<TransactionWithOutcome>,
    pub receipts: Vec<views::ReceiptView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionWithOutcome {
    pub transaction: views::SignedTransactionView,
    pub outcome: ExecutionOutcomeWithOptionalReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionOutcomeWithOptionalReceipt {
    pub execution_outcome: views::ExecutionOutcomeWithIdView,
    pub receipt: Option<views::ReceiptView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionOutcomeWithReceipt {
    pub execution_outcome: views::ExecutionOutcomeWithIdView,
    pub receipt: views::ReceiptView,
}
