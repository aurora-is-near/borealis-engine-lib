use crate::utils::{u64_hex_serde, u128_dec_serde};
use aurora_engine::parameters::{ResultLog, TransactionStatus};
use aurora_engine_transactions::eip_2930::AccessTuple;
use aurora_engine_types::types::{Address, Wei};
use aurora_engine_types::{H256, U256};
use derive_builder::Builder;
use near_primitives::hash::CryptoHash;
use near_primitives::types::AccountId;
use serde::{Deserialize, Serialize};

use crate::bloom::Bloom;

/// Similar to Ethereum blocks, but only contains information relevant for Aurora. In addition,
/// it contains extra metadata to map it into a NEAR block.
///
/// ## Fields from Ethereum blocks not included:
///
/// baseFeePerGas, difficulty, miner, mixHash, nonce, totalDifficulty, uncles,
///
/// ## Skip blocks:
///
/// Note that some blocks on NEAR are skipped, and in this case we are creating a boilerplate block
/// with unique hash, and consistent parent_hash and height.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuroraBlock {
    /// Chain where this block belongs to
    pub chain_id: u64,
    /// Account id of the engine contract on the chain
    pub engine_account_id: AccountId,
    /// Hash of the block
    pub hash: H256,
    /// Hash of the parent block. It is guaranteed that heights from consecutive blocks will be
    /// consecutive. i.e: block(parent_hash).height + 1 == block(hash)
    pub parent_hash: H256,
    /// Height of the block. This height matches the NEAR
    pub height: u64,
    /// Implicit account id of the NEAR validator that mined this block
    pub miner: Address,
    /// Timestamp where the block was generated
    pub timestamp: u64,
    /// Gas limit will be always u64::MAX
    #[serde(with = "u64_hex_serde")]
    pub gas_limit: u64,
    /// Sum of the gas used for each tx included in the block.
    #[serde(with = "u64_hex_serde")]
    pub gas_used: u64,
    /// Logs bloom of the block. Aggregation of transactions logs bloom.
    pub logs_bloom: Bloom,
    /// Integer the size of this block in bytes.
    #[serde(with = "u64_hex_serde")]
    pub size: u64,
    /// Transaction root using Ethereum rules
    pub transactions_root: H256,
    /// State root: Uses NEAR state root of the block. While this doesn't match Ethereum rules to compute
    /// proofs, it contains the relevant information to make any proof about any piece of state in Aurora.
    /// Note however that the state root included in block X matches the previous block. This means that
    /// at block X you can only build proofs of events that happened prior the execution of that block.
    pub state_root: H256,
    /// Receipts root using Ethereum rules
    pub receipts_root: H256,
    /// List with all txs in the current block. Txs will be extracted from the receipts executed in
    /// a block. This means that potentially the original NEAR tx could have been created in an
    /// older block, but it was executed in the current block. For NEAR txs that create several
    /// contract calls, potentially hitting aurora several times, a different Ethereum tx will be
    /// created for each receipt.
    pub transactions: Vec<AuroraTransaction>,
    /// Metadata to recover the block on NEAR
    pub near_metadata: NearBlock,
}

/// Near block metadata
#[derive(Debug, Serialize, Deserialize)]
pub enum NearBlock {
    /// No block is known at this height.
    SkipBlock,
    /// Metadata from an existing block.
    ExistingBlock(NearBlockHeader),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NearBlockHeader {
    /// Hash of the block on NEAR
    pub near_hash: CryptoHash,
    /// Hash of the parent of block on NEAR. Note that some blocks can be skipped.
    pub near_parent_hash: CryptoHash,
    /// Account id of the validator that produced this block
    pub author: AccountId,
}

/// Similar to Ethereum transaction but only contains information relevant for Aurora.
///
/// It includes the information of the receipt after executing the transaction as well. In addition it contains
/// extra metadata to map it into a NEAR transaction.
///
/// ## Fields from Ethereum transactions and receipts not included:
///
/// `cumulativeGasUsed, effectiveGasPrice, logsBloom`
#[derive(Builder, Debug, Serialize, Deserialize)]
#[builder(pattern = "owned")]
pub struct AuroraTransaction {
    /// Hash of the transaction and the receipt
    pub hash: H256,
    /// Hash of the block where the transaction was included.
    pub block_hash: H256,
    /// Height of the block where the transaction was included.
    pub block_height: u64,
    /// Target chain id of the transaction.
    pub chain_id: u64,
    /// Index of the transaction on the block. This index is computed after filtering out all
    /// transactions that are not relevant to current aurora chain id.
    pub transaction_index: u32,
    /// Sender of the transaction. If the transaction is not sent via submit, the sender will be
    /// derived using `near_account_to_evm_address`.
    pub from: Address,
    /// Target address of the transaction. It will be None in case it is a deploy transaction.
    pub to: Option<Address>,
    /// Nonce of the transaction to keep the order.
    #[serde(with = "u64_hex_serde")]
    pub nonce: u64,
    /// Gas price for the transaction. Related to Aurora Gas not NEAR Gas.
    pub gas_price: U256,
    /// Gas limit of the transaction. In the context of Aurora it should be U256::MAX
    #[serde(with = "u64_hex_serde")]
    pub gas_limit: u64,
    /// Gas used by the transaction
    pub gas_used: u64,
    pub max_priority_fee_per_gas: U256,
    pub max_fee_per_gas: U256,
    /// Amount of eth attached to the transaction.
    pub value: Wei,
    /// Input of the transaction passed to the target contract.
    pub input: Vec<u8>,
    /// Output of the transaction. The result from the execution.
    pub output: Vec<u8>,
    /// List of addresses that will be used during execution of the transaction.
    pub access_list: Vec<AccessTuple>,
    /// Type format of the transaction.
    pub tx_type: u8,
    /// Status of the transaction execution.
    pub status: bool,
    /// Logs recorded during transaction execution. For now they will be empty, since it can't be
    /// computed without access to the storage.
    pub logs: Vec<ResultLog>,
    /// Logs bloom of the transaction. Aggregation of bloom filters from logs
    pub logs_bloom: Bloom,
    /// Address of the deployed contract. If will be different from `None` in case it is a deploy
    /// transaction.
    pub contract_address: Option<Address>,
    /// Signature data. Used to recover target address.
    pub v: u64,
    pub r: U256,
    pub s: U256,
    /// Metadata to recover the NEAR transaction/receipt associated with this transaction
    pub near_metadata: NearTransaction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NearTransaction {
    /// Index of the action on action list
    pub action_index: usize,
    /// Receipt hash on NEAR
    pub receipt_hash: CryptoHash,
    /// Transaction hash on NEAR that caused the receipt to be produced
    /// (potentially indirectly via a number of other receipts).
    /// Field is optional for backwards compatibility, but is
    /// expected to be set for call newly produced data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_hash: Option<CryptoHash>,
    /// Transaction metadata which is needed to compute the
    /// value this transaction contributes to the overall block hashchain.
    /// See [AIP-008](https://github.com/aurora-is-near/AIPs/pull/8) for details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hashchain_metadata: Option<HashchainMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HashchainMetadata {
    /// Name of the method called on the Engine in the original Near transaction.
    pub method_name: String,
    /// Tag describing how to compute the original Near input from the data available in `AuroraTransaction`.
    pub input: HashchainInputKind,
    /// Tag describing how to compute the original Near output from the data available in `AuroraTransaction`.
    pub output: HashchainOutputKind,
    /// Computed "intrinsic transaction hash" as per
    /// [AIP-008](https://github.com/aurora-is-near/AIPs/pull/8)
    /// using the transaction data as described in the above fields.
    /// This is present as a sort of checksum to ensure the overall `AuroraTransaction`
    /// is consistent with what was present in the original Near transaction.
    pub intrinsic_hash: CryptoHash,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HashchainInputKind {
    /// Standard RLP-encoded Ethereum transaction.
    Rlp,
    /// Legacy version of Borsh-encoded args for `call` function.
    CallArgsLegacy,
    /// Current version of Borsh-encoded args for `call` function.
    CallArgs(CallArgsVersion),
    /// The `submit_with_args` function uses a Borsh-encoded struct that has an embedded
    /// RLP-encoded Ethereum transaction along with additional parameters.
    SubmitWithArgs(AdditionalSubmitArgs),
    /// Raw input explicitly given in `AuroraTransaction.input`
    Explicit,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CallArgsVersion {
    V1,
    V2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdditionalSubmitArgs {
    #[serde(with = "u128_dec_serde")]
    pub max_gas_price: Option<u128>,
    pub gas_token_address: Option<Address>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HashchainOutputKind {
    /// Borsh-encoding of SubmitResultLegacyV1 (fields populated with data from `AuroraTransaction`).
    SubmitResultLegacyV1(ResultStatusTag),
    /// Borsh-encoding of SubmitResultLegacyV2 (fields populated with data from `AuroraTransaction`).
    SubmitResultLegacyV2(ResultStatusTag),
    /// Borsh-encoding of SubmitResultLegacyV3 (fields populated with data from `AuroraTransaction`).
    SubmitResultLegacyV3,
    /// Borsh-encoding of current SubmitResult version (fields populated with data from `AuroraTransaction`).
    SubmitResultV7(ResultStatusTag),
    /// Raw output explicitly given in `AuroraTransaction.output`
    Explicit,
    /// No output from this transaction on Near (because another receipt was created instead).
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ResultStatusTag {
    Success,
    Revert,
    OutOfGas,
    OutOfFund,
    OutOfOffset,
    CallTooDeep,
    StackUnderflow,
    StackOverflow,
    InvalidJump,
    InvalidRange,
    DesignatedInvalid,
    CreateCollision,
    CreateContractLimit,
    InvalidCode(u8),
    PCUnderflow,
    CreateEmpty,
    MaxNonce,
    UsizeOverflow,
    CreateContractStartingWithEF,
    Other(String),
}

impl From<&TransactionStatus> for ResultStatusTag {
    fn from(value: &TransactionStatus) -> Self {
        match value {
            TransactionStatus::Succeed(_) => Self::Success,
            TransactionStatus::Revert(_) => Self::Revert,
            TransactionStatus::OutOfGas => Self::OutOfGas,
            TransactionStatus::OutOfFund => Self::OutOfFund,
            TransactionStatus::OutOfOffset => Self::OutOfOffset,
            TransactionStatus::CallTooDeep => Self::CallTooDeep,
            TransactionStatus::StackUnderflow => Self::StackUnderflow,
            TransactionStatus::StackOverflow => Self::StackOverflow,
            TransactionStatus::InvalidJump => Self::InvalidJump,
            TransactionStatus::InvalidRange => Self::InvalidRange,
            TransactionStatus::DesignatedInvalid => Self::DesignatedInvalid,
            TransactionStatus::CreateCollision => Self::CreateCollision,
            TransactionStatus::CreateContractLimit => Self::CreateContractLimit,
            TransactionStatus::InvalidCode(code) => Self::InvalidCode(*code),
            TransactionStatus::PCUnderflow => Self::PCUnderflow,
            TransactionStatus::CreateEmpty => Self::CreateEmpty,
            TransactionStatus::MaxNonce => Self::MaxNonce,
            TransactionStatus::UsizeOverflow => Self::UsizeOverflow,
            TransactionStatus::Other(msg) => Self::Other(msg.to_string()),
            TransactionStatus::CreateContractStartingWithEF => Self::CreateContractStartingWithEF,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AuroraBlock;

    #[test]
    fn test_aurora_block_deserialization() {
        let serialized_block = include_str!("../tests/res/aurora_block/aurora_70077007.json");
        let given_block_json: serde_json::Value = serde_json::from_str(serialized_block).unwrap();

        let block: AuroraBlock = serde_json::from_str(serialized_block).unwrap();
        let computed_block_json = {
            let mut tmp = serde_json::to_value(block).unwrap();
            // The gas limit field used to be equal to `U256::MAX`, however we now represent that
            // field as `u64`, so it is automatically coerced down to `u64::MAX` when deserializing.
            // Therefore, we expect the value now to be `0xffffffffffffffff`, but it used to be
            // `0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`, so we change it back
            // to be able to compare with the original given block.
            assert_eq!(
                tmp.as_object()
                    .unwrap()
                    .get("gas_limit")
                    .unwrap()
                    .as_str()
                    .unwrap(),
                "0xffffffffffffffff"
            );
            tmp.as_object_mut().unwrap().insert(
                "gas_limit".into(),
                serde_json::Value::String(
                    "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into(),
                ),
            );
            tmp
        };

        assert_eq!(computed_block_json, given_block_json);
    }
}
