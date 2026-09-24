//! The block data consumed by the engine, transaction tracker, and refiner.
//!
//! Convert an incoming nearcore `StreamerMessage` with `InnerNearBlock::try_from`,
//! or decode NEAR block JSON with `InnerNearBlock::from_bytes`. Both
//! paths preserve the Borsh data required by Aurora output for known protocol
//! variants. Call `validate_for_engine` before producing an Aurora block: future
//! receipts for other accounts can remain opaque, but Aurora receipts need an
//! exact Borsh size. The reduced public model contains no NEAR views; accounts
//! and hashes retain the existing primitive types for interoperability.

use near_primitives::{hash::CryptoHash, types::AccountId};
use serde::{Deserialize, Serialize, de};
use std::io;

use crate::inner_block::wire::RawBlockMessage;
pub use wire::ConversionError;

mod borsh_compat;
mod nearcore;
#[cfg(test)]
mod tests;
mod wire;

/// Stable internal block model. Its standard serde representation is symmetric
/// and distinct from the external NEAR schema.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct InnerNearBlock {
    pub block: Block,
    pub shards: Vec<Shard>,
}

impl InnerNearBlock {
    /// Decodes one complete NEAR block message from JSON bytes.
    pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> serde_json::Result<Self> {
        let message = serde_json::from_slice::<RawBlockMessage>(bytes.as_ref())?;
        Self::try_from(message).map_err(de::Error::custom)
    }

    /// Checks the fields whose exact NEAR Borsh representation is required to
    /// produce a block for this engine account, and that every storage change of
    /// the engine account can be attributed to the receipt that caused it.
    pub fn validate_for_engine(&self, engine_account_id: &str) -> Result<(), ConversionError> {
        for outcome in self
            .shards
            .iter()
            .flat_map(|shard| &shard.receipt_execution_outcomes)
            .filter(|outcome| outcome.receipt.receiver_id.as_str() == engine_account_id)
        {
            let reason = match (&outcome.receipt.receipt, outcome.receipt_size) {
                (ReceiptKind::Unsupported(reason), _) => Some(reason.as_str()),
                (_, None) => Some("missing receipt Borsh size"),
                _ => None,
            };
            if let Some(reason) = reason {
                return Err(ConversionError::new(
                    self.block.header.height,
                    self.block.header.hash,
                    io::Error::other(format!(
                        "cannot decode Aurora receipt {}: {reason}",
                        outcome.receipt.receipt_id
                    )),
                ));
            }
            if let ExecutionStatus::Unsupported(reason) = &outcome.execution_outcome.status {
                return Err(ConversionError::new(
                    self.block.header.height,
                    self.block.header.hash,
                    io::Error::other(format!(
                        "cannot decode Aurora receipt {} execution status: {reason}",
                        outcome.receipt.receipt_id
                    )),
                ));
            }
        }
        for change in self
            .shards
            .iter()
            .flat_map(|shard| &shard.state_changes)
            .filter(|change| change.account_id.as_str() == engine_account_id)
        {
            if let StateChangeCause::Other(cause) = &change.cause {
                return Err(ConversionError::new(
                    self.block.header.height,
                    self.block.header.hash,
                    io::Error::other(format!(
                        "unsupported cause of Aurora storage change: {cause}"
                    )),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Block {
    pub author: AccountId,
    pub header: BlockHeader,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct BlockHeader {
    pub height: u64,
    pub hash: CryptoHash,
    pub prev_hash: CryptoHash,
    pub prev_state_root: CryptoHash,
    /// Preserve the timestamp used in Aurora output, independently of the engine timestamp.
    pub timestamp: u64,
    pub timestamp_nanosec: u64,
    pub random_value: CryptoHash,
    /// Number of entries in the source chunk mask, used to detect missing shards.
    pub expected_shards: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Shard {
    pub chunk: Option<Chunk>,
    pub receipt_execution_outcomes: Vec<ReceiptExecutionOutcome>,
    pub state_changes: Vec<StateChange>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Chunk {
    /// Includes all transactions, even those that do not directly call Aurora.
    pub transactions: Vec<Transaction>,
    pub receipts: Vec<ChunkReceipt>,
    /// Kept separate so consumers can process local receipts before incoming ones.
    pub local_receipts: Vec<ChunkReceipt>,
}

/// The transaction-to-receipt links needed to recover transaction provenance.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Transaction {
    pub hash: CryptoHash,
    pub receipt_ids: Vec<CryptoHash>,
}

/// A chunk receipt's position and optional promise result; actions are consumed
/// from execution outcomes instead.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ChunkReceipt {
    pub receiver_id: AccountId,
    pub receipt_id: CryptoHash,
    pub data: Option<DataReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DataReceipt {
    pub data_id: CryptoHash,
    /// `None` is a failed promise, distinct from a successful empty result.
    pub data: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ReceiptExecutionOutcome {
    pub execution_outcome: ExecutionOutcome,
    pub receipt: Receipt,
    /// Borsh size of the original NEAR receipt, used in AuroraBlock::size.
    /// Unknown external receipts have no recoverable size and must not be
    /// processed as Aurora receipts.
    pub receipt_size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExecutionOutcome {
    pub logs: Vec<String>,
    pub receipt_ids: Vec<CryptoHash>,
    pub status: ExecutionStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum ExecutionStatus {
    Unknown,
    /// Original Borsh-encoded execution error, for custom transaction output.
    Failure(Vec<u8>),
    /// A NEAR failure unknown to the pinned error codec. Its failed status is
    /// still usable; code that needs exact Borsh bytes must reject it.
    UnencodedFailure(String),
    /// A future or malformed status with unknown success semantics.
    Unsupported(String),
    SuccessValue(Vec<u8>),
    SuccessReceiptId(CryptoHash),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Receipt {
    pub predecessor_id: AccountId,
    pub receiver_id: AccountId,
    pub receipt_id: CryptoHash,
    pub receipt: ReceiptKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum ReceiptKind {
    Action {
        signer_id: AccountId,
        input_data_ids: Vec<CryptoHash>,
        /// Preserve every action and its position, including non-function calls.
        actions: Vec<Action>,
    },
    Data {
        data_id: CryptoHash,
    },
    GlobalContractDistribution,
    /// A future or malformed external receipt. Provenance links remain usable,
    /// but Aurora receipts of this kind must fail before block production.
    Unsupported(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Action {
    FunctionCall {
        method_name: String,
        args: Vec<u8>,
        /// Attached balance in yoctoNEAR.
        deposit: u128,
    },
    /// A known NEAR action represented by its original Borsh bytes for a custom
    /// Aurora transaction. This is not a fallback for unknown JSON variants.
    Other { borsh_bytes: Vec<u8> },
}

/// A contract storage update. The engine does not use other NEAR state changes.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct StateChange {
    pub account_id: AccountId,
    pub key: Vec<u8>,
    /// `None` deletes the key; `Some(vec![])` stores an empty value.
    pub value: Option<Vec<u8>>,
    pub cause: StateChangeCause,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum StateChangeCause {
    ReceiptProcessing {
        receipt_hash: CryptoHash,
    },
    /// Keep the cause for diagnostics. `InnerNearBlock::validate_for_engine`
    /// rejects it for the engine account rather than silently dropping its
    /// storage diff.
    Other(String),
}
