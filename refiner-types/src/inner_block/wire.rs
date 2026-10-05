use borsh::BorshSerialize;
use near_crypto::{PublicKey, Signature};
use near_primitives::{
    hash::CryptoHash,
    serialize::dec_format,
    types::{AccountId, Balance, Gas, ShardId, StoreKey, StoreValue},
};
use serde::{Deserialize, de};
use serde_json::value::RawValue;
use serde_with::{base64::Base64, serde_as};
use std::{collections::BTreeMap, fmt, io};

use super::{
    Action, Block, BlockHeader, Chunk, ChunkReceipt, DataReceipt, ExecutionOutcome,
    ExecutionStatus, InnerNearBlock, Receipt, ReceiptExecutionOutcome, ReceiptKind, Shard,
    StateChange, StateChangeCause, Transaction,
    borsh_compat::{self, DelegateActionCompat, VersionedDelegateActionPayloadCompat},
};

/// Failure to preserve the Borsh representation required by Aurora output.
#[derive(Debug)]
pub struct ConversionError {
    pub block_height: u64,
    pub block_hash: CryptoHash,
    source: io::Error,
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to convert NEAR block {} ({}): {}",
            self.block_height, self.block_hash, self.source
        )
    }
}

impl std::error::Error for ConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl ConversionError {
    pub(super) const fn new(block_height: u64, block_hash: CryptoHash, source: io::Error) -> Self {
        Self {
            block_height,
            block_hash,
            source,
        }
    }
}

impl TryFrom<RawBlockMessage> for InnerNearBlock {
    type Error = ConversionError;

    fn try_from(source: RawBlockMessage) -> Result<Self, Self::Error> {
        let header = source.block.header;
        let shards = source
            .shards
            .into_iter()
            .map(Shard::try_from)
            .collect::<Result<_, _>>()
            .map_err(|source| ConversionError::new(header.height, header.hash, source))?;

        Ok(Self {
            block: Block {
                author: source.block.author,
                header: BlockHeader {
                    height: header.height,
                    hash: header.hash,
                    prev_hash: header.prev_hash,
                    prev_state_root: header.prev_state_root,
                    timestamp: header.timestamp,
                    timestamp_nanosec: header.timestamp_nanosec,
                    random_value: header.random_value,
                    expected_shards: header.chunk_mask.len(),
                },
            },
            shards,
        })
    }
}

impl TryFrom<RawShard> for Shard {
    type Error = io::Error;

    fn try_from(source: RawShard) -> Result<Self, Self::Error> {
        Ok(Self {
            chunk: source.chunk.map(|source| Chunk {
                transactions: source
                    .transactions
                    .into_iter()
                    .map(|tx| Transaction {
                        hash: tx.transaction.hash,
                        receipt_ids: tx.outcome.execution_outcome.outcome.receipt_ids,
                    })
                    .collect(),
                receipts: source
                    .receipts
                    .into_iter()
                    .map(convert_raw_chunk_receipt)
                    .collect(),
                local_receipts: source
                    .local_receipts
                    .into_iter()
                    .map(convert_raw_chunk_receipt)
                    .collect(),
            }),
            receipt_execution_outcomes: source
                .receipt_execution_outcomes
                .into_iter()
                .map(convert_raw_receipt_execution_outcome)
                .collect::<Result<_, _>>()?,
            state_changes: source
                .state_changes
                .into_iter()
                .map(|source| {
                    let cause = source.cause.try_into()?;
                    Ok(convert_raw_state_change(source.value, cause))
                })
                .collect::<Result<Vec<_>, io::Error>>()?
                .into_iter()
                .flatten()
                .collect(),
        })
    }
}

fn convert_raw_chunk_receipt(source: RawChunkReceipt) -> ChunkReceipt {
    let data = match source.receipt {
        RawChunkReceiptKind::Data(receipt) => Some(DataReceipt {
            data_id: receipt.data_id,
            data: receipt.data,
        }),
        RawChunkReceiptKind::Other => None,
    };
    ChunkReceipt {
        receiver_id: source.receiver_id,
        receipt_id: source.receipt_id,
        data,
    }
}

/// The payload of `UniversalStateInit` is opaque protocol bytes. Its JSON form
/// is base64, while Borsh writes the byte vector with its length prefix.
#[serde_as]
#[derive(BorshSerialize, Clone, Deserialize)]
struct RawStateInit(#[serde_as(as = "Base64")] Vec<u8>);

fn convert_raw_receipt_execution_outcome(
    source: RawExecutionOutcomeWithReceipt,
) -> Result<ReceiptExecutionOutcome, io::Error> {
    let RawReceiptEnvelope {
        predecessor_id,
        receiver_id,
        receipt_id,
        receipt,
        priority,
    } = source.receipt;
    let encoded = serde_json::from_str::<RawReceiptKind>(receipt.get())
        .map_err(io::Error::other)
        .and_then(|receipt| {
            let receipt_size = borsh::object_length(&RawReceipt {
                predecessor_id: predecessor_id.clone(),
                receiver_id: receiver_id.clone(),
                receipt_id,
                receipt: receipt.clone(),
                priority,
            })? as u64;
            Ok((receipt.try_into()?, receipt_size))
        });
    let (receipt, receipt_size) = match encoded {
        Ok((receipt, receipt_size)) => (receipt, Some(receipt_size)),
        Err(error) => (ReceiptKind::Unsupported(error.to_string()), None),
    };
    let outcome = source.execution_outcome.outcome;
    Ok(ReceiptExecutionOutcome {
        execution_outcome: ExecutionOutcome {
            logs: outcome.logs,
            receipt_ids: outcome.receipt_ids,
            status: outcome.status.try_into()?,
        },
        receipt: Receipt {
            predecessor_id,
            receiver_id,
            receipt_id,
            receipt,
        },
        receipt_size,
    })
}

impl TryFrom<RawExecutionStatus> for ExecutionStatus {
    type Error = io::Error;

    fn try_from(source: RawExecutionStatus) -> Result<Self, Self::Error> {
        Ok(match source {
            RawExecutionStatus::Known(RawKnownExecutionStatus::Unknown) => Self::Unknown,
            RawExecutionStatus::Known(RawKnownExecutionStatus::Failure(error)) => {
                borsh_compat::execution_error_bytes(&error)?.map_or_else(
                    || Self::UnencodedFailure(error.get().to_string()),
                    Self::Failure,
                )
            }
            RawExecutionStatus::Known(RawKnownExecutionStatus::SuccessValue(value)) => {
                Self::SuccessValue(value)
            }
            RawExecutionStatus::Known(RawKnownExecutionStatus::SuccessReceiptId(id)) => {
                Self::SuccessReceiptId(id)
            }
            RawExecutionStatus::Unsupported(raw) => Self::Unsupported(raw),
        })
    }
}

impl TryFrom<RawReceiptKind> for ReceiptKind {
    type Error = io::Error;

    fn try_from(source: RawReceiptKind) -> Result<Self, Self::Error> {
        Ok(match source {
            RawReceiptKind::Action {
                signer_id,
                input_data_ids,
                actions,
                ..
            } => Self::Action {
                signer_id,
                input_data_ids,
                actions: actions
                    .into_iter()
                    .map(Action::try_from)
                    .collect::<Result<_, _>>()?,
            },
            RawReceiptKind::Data { data_id, .. } => Self::Data { data_id },
            RawReceiptKind::GlobalContractDistribution { .. } => Self::GlobalContractDistribution,
        })
    }
}

impl TryFrom<RawAction> for Action {
    type Error = io::Error;

    fn try_from(source: RawAction) -> Result<Self, Self::Error> {
        Ok(match source {
            RawAction::FunctionCall {
                method_name,
                args,
                deposit,
                ..
            } => Self::FunctionCall {
                method_name,
                args,
                deposit: deposit.as_yoctonear(),
            },
            action => Self::Other {
                borsh_bytes: borsh::to_vec(&action)?,
            },
        })
    }
}

fn convert_raw_state_change(
    value: RawStateChangeValue,
    cause: StateChangeCause,
) -> Option<StateChange> {
    let (account_id, key, value) = match value {
        RawStateChangeValue::DataUpdate { change } => (
            change.account_id,
            change.key.into(),
            Some(change.value.into()),
        ),
        RawStateChangeValue::DataDeletion { change } => {
            (change.account_id, change.key.into(), None)
        }
        RawStateChangeValue::Other => return None,
    };
    Some(StateChange {
        account_id,
        key,
        value,
        cause,
    })
}

#[derive(Clone, Deserialize)]
pub struct RawBlockMessage {
    block: RawBlock,
    shards: Vec<RawShard>,
}

#[derive(Clone, Deserialize)]
struct RawBlock {
    author: AccountId,
    header: RawBlockHeader,
}

#[derive(Clone, Deserialize)]
struct RawBlockHeader {
    height: u64,
    hash: CryptoHash,
    prev_hash: CryptoHash,
    prev_state_root: CryptoHash,
    timestamp: u64,
    #[serde(with = "dec_format")]
    timestamp_nanosec: u64,
    random_value: CryptoHash,
    chunk_mask: Vec<bool>,
}

#[derive(Clone, Deserialize)]
struct RawShard {
    chunk: Option<RawChunk>,
    receipt_execution_outcomes: Vec<RawExecutionOutcomeWithReceipt>,
    state_changes: Vec<RawStateChange>,
}

#[derive(Clone, Deserialize)]
struct RawChunk {
    transactions: Vec<RawTransactionWithOutcome>,
    receipts: Vec<RawChunkReceipt>,
    #[serde(default)]
    local_receipts: Vec<RawChunkReceipt>,
}

#[derive(Clone, Deserialize)]
struct RawTransactionWithOutcome {
    transaction: RawTransaction,
    outcome: RawExecutionOutcomeWithOptionalReceipt,
}

#[derive(Clone, Deserialize)]
struct RawTransaction {
    hash: CryptoHash,
}

#[derive(Clone, Deserialize)]
struct RawExecutionOutcomeWithOptionalReceipt {
    execution_outcome: RawTransactionExecutionOutcome,
}

#[derive(Clone, Deserialize)]
struct RawTransactionExecutionOutcome {
    outcome: RawTransactionOutcome,
}

#[derive(Clone, Deserialize)]
struct RawTransactionOutcome {
    receipt_ids: Vec<CryptoHash>,
}

#[derive(Clone, Deserialize)]
struct RawExecutionOutcomeWithReceipt {
    execution_outcome: RawExecutionOutcomeWithId,
    receipt: RawReceiptEnvelope,
}

#[derive(Clone, Deserialize)]
struct RawExecutionOutcomeWithId {
    outcome: RawExecutionOutcome,
}

#[derive(Clone, Deserialize)]
struct RawExecutionOutcome {
    logs: Vec<String>,
    receipt_ids: Vec<CryptoHash>,
    status: RawExecutionStatus,
}

/// Local projection of the execution status encoded in NEAR block JSON.
#[derive(Clone)]
enum RawExecutionStatus {
    Known(RawKnownExecutionStatus),
    Unsupported(String),
}

impl<'de> Deserialize<'de> for RawExecutionStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        Ok(serde_json::from_str(raw.get())
            .map_or_else(|_| Self::Unsupported(raw.get().to_string()), Self::Known))
    }
}

#[serde_as]
#[derive(Clone, Deserialize)]
enum RawKnownExecutionStatus {
    Unknown,
    Failure(Box<RawValue>),
    SuccessValue(#[serde_as(as = "Base64")] Vec<u8>),
    SuccessReceiptId(CryptoHash),
}

/// Local receipt wire type. Its Borsh field order must match NEAR receipts
/// because `receipt_size` is part of Aurora block output.
#[derive(BorshSerialize, Clone)]
struct RawReceipt {
    predecessor_id: AccountId,
    receiver_id: AccountId,
    receipt_id: CryptoHash,
    receipt: RawReceiptKind,
    priority: u64,
}

#[derive(Clone, Deserialize)]
struct RawReceiptEnvelope {
    predecessor_id: AccountId,
    receiver_id: AccountId,
    receipt_id: CryptoHash,
    receipt: Box<RawValue>,
    #[serde(default)]
    priority: u64,
}

#[derive(Clone, Deserialize)]
struct RawChunkReceipt {
    receiver_id: AccountId,
    receipt_id: CryptoHash,
    receipt: RawChunkReceiptKind,
}

#[derive(Clone)]
enum RawChunkReceiptKind {
    Data(RawChunkDataReceipt),
    Other,
}

impl<'de> Deserialize<'de> for RawChunkReceiptKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = RawChunkReceiptKind;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a receipt object containing one variant")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let variant: String = map
                    .next_key()?
                    .ok_or_else(|| de::Error::custom("chunk receipt has no variant"))?;
                let receipt = if variant == "Data" {
                    RawChunkReceiptKind::Data(map.next_value()?)
                } else {
                    map.next_value::<de::IgnoredAny>()?;
                    RawChunkReceiptKind::Other
                };
                if map.next_key::<de::IgnoredAny>()?.is_some() {
                    return Err(de::Error::custom("chunk receipt has multiple variants"));
                }
                Ok(receipt)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

#[serde_as]
#[derive(Clone, Deserialize)]
struct RawChunkDataReceipt {
    data_id: CryptoHash,
    #[serde_as(as = "Option<Base64>")]
    data: Option<Vec<u8>>,
}

/// Local projection of a NEAR receipt. Explicit discriminants preserve the
/// Borsh representation without deserializing through `near_primitives::views`.
#[serde_as]
#[derive(BorshSerialize, Clone, Deserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
enum RawReceiptKind {
    Action {
        signer_id: AccountId,
        signer_public_key: PublicKey,
        gas_price: Balance,
        output_data_receivers: Vec<RawDataReceiver>,
        input_data_ids: Vec<CryptoHash>,
        actions: Vec<RawAction>,
        #[serde(default)]
        is_promise_yield: bool,
        #[serde(default)]
        refund_to: Option<AccountId>,
    } = 0,
    Data {
        data_id: CryptoHash,
        #[serde_as(as = "Option<Base64>")]
        data: Option<Vec<u8>>,
        #[serde(default)]
        is_promise_resume: bool,
    } = 1,
    GlobalContractDistribution {
        id: RawGlobalContractIdentifier,
        target_shard: ShardId,
        already_delivered_shards: Vec<ShardId>,
        #[serde_as(as = "Base64")]
        code: Vec<u8>,
        #[serde(default)]
        nonce: Option<u64>,
    } = 2,
}

#[derive(BorshSerialize, Clone, Deserialize)]
struct RawDataReceiver {
    data_id: CryptoHash,
    receiver_id: AccountId,
}

/// Local projection of a NEAR action. Besides `FunctionCall`, the complete
/// action is retained as Borsh bytes for Aurora custom transaction output.
#[serde_as]
#[derive(BorshSerialize, Clone, Deserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
enum RawAction {
    CreateAccount = 0,
    DeployContract {
        #[serde_as(as = "Base64")]
        code: Vec<u8>,
    } = 1,
    FunctionCall {
        method_name: String,
        #[serde_as(as = "Base64")]
        args: Vec<u8>,
        gas: Gas,
        deposit: Balance,
    } = 2,
    Transfer {
        deposit: Balance,
    } = 3,
    Stake {
        stake: Balance,
        public_key: PublicKey,
    } = 4,
    AddKey {
        public_key: PublicKey,
        access_key: RawAccessKey,
    } = 5,
    DeleteKey {
        public_key: PublicKey,
    } = 6,
    DeleteAccount {
        beneficiary_id: AccountId,
    } = 7,
    Delegate {
        delegate_action: DelegateActionCompat,
        signature: Signature,
    } = 8,
    DeployGlobalContract {
        #[serde_as(as = "Base64")]
        code: Vec<u8>,
    } = 9,
    DeployGlobalContractByAccountId {
        #[serde_as(as = "Base64")]
        code: Vec<u8>,
    } = 10,
    UseGlobalContract {
        code_hash: CryptoHash,
    } = 11,
    UseGlobalContractByAccountId {
        account_id: AccountId,
    } = 12,
    DeterministicStateInit {
        code: RawGlobalContractIdentifier,
        #[serde_as(as = "BTreeMap<Base64, Base64>")]
        data: BTreeMap<Vec<u8>, Vec<u8>>,
        deposit: Balance,
    } = 13,
    TransferToGasKey {
        public_key: PublicKey,
        deposit: Balance,
    } = 14,
    WithdrawFromGasKey {
        public_key: PublicKey,
        amount: Balance,
    } = 15,
    DelegateV2 {
        delegate_action: VersionedDelegateActionPayloadCompat,
        signature: Signature,
    } = 16,
    UniversalStateInit {
        state_init: RawStateInit,
        deposit: Balance,
    } = 17,
}

#[derive(BorshSerialize, Clone, Deserialize)]
struct RawAccessKey {
    nonce: u64,
    permission: RawAccessKeyPermission,
}

#[derive(BorshSerialize, Clone, Deserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
enum RawAccessKeyPermission {
    FunctionCall {
        allowance: Option<Balance>,
        receiver_id: String,
        method_names: Vec<String>,
    } = 0,
    FullAccess = 1,
    GasKeyFunctionCall {
        balance: Balance,
        num_nonces: u16,
        allowance: Option<Balance>,
        receiver_id: String,
        method_names: Vec<String>,
    } = 2,
    GasKeyFullAccess {
        balance: Balance,
        num_nonces: u16,
    } = 3,
}

#[derive(BorshSerialize, Clone)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
enum RawGlobalContractIdentifier {
    CodeHash(CryptoHash) = 0,
    AccountId(AccountId) = 1,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawGlobalContractIdentifierJson {
    CodeHash { hash: CryptoHash },
    AccountId { account_id: AccountId },
    DeprecatedCodeHash(CryptoHash),
    DeprecatedAccountId(AccountId),
}

impl<'de> Deserialize<'de> for RawGlobalContractIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(
            match RawGlobalContractIdentifierJson::deserialize(deserializer)? {
                RawGlobalContractIdentifierJson::CodeHash { hash }
                | RawGlobalContractIdentifierJson::DeprecatedCodeHash(hash) => Self::CodeHash(hash),
                RawGlobalContractIdentifierJson::AccountId { account_id }
                | RawGlobalContractIdentifierJson::DeprecatedAccountId(account_id) => {
                    Self::AccountId(account_id)
                }
            },
        )
    }
}

#[derive(Clone, Deserialize)]
struct RawStateChangeCause {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    tx_hash: Option<String>,
    #[serde(default)]
    receipt_hash: Option<String>,
}

impl TryFrom<RawStateChangeCause> for StateChangeCause {
    type Error = io::Error;

    fn try_from(source: RawStateChangeCause) -> Result<Self, Self::Error> {
        let require_hash = |hash: Option<String>, field| -> Result<CryptoHash, io::Error> {
            let hash = hash
                .ok_or_else(|| io::Error::other(format!("missing {field} for {}", source.kind)))?;
            hash.parse().map_err(|error| {
                io::Error::other(format!("invalid {field} for {}: {error}", source.kind))
            })
        };

        Ok(match source.kind.as_str() {
            "receipt_processing" => Self::ReceiptProcessing {
                receipt_hash: require_hash(source.receipt_hash, "receipt_hash")?,
            },
            "transaction_processing" => {
                let tx_hash = require_hash(source.tx_hash, "tx_hash")?;
                Self::Other(format!("TransactionProcessing {{ tx_hash: {tx_hash:?} }}"))
            }
            "action_receipt_processing_started" => {
                let receipt_hash = require_hash(source.receipt_hash, "receipt_hash")?;
                Self::Other(format!(
                    "ActionReceiptProcessingStarted {{ receipt_hash: {receipt_hash:?} }}"
                ))
            }
            "action_receipt_gas_reward" => {
                let receipt_hash = require_hash(source.receipt_hash, "receipt_hash")?;
                Self::Other(format!(
                    "ActionReceiptGasReward {{ receipt_hash: {receipt_hash:?} }}"
                ))
            }
            "postponed_receipt" => {
                let receipt_hash = require_hash(source.receipt_hash, "receipt_hash")?;
                Self::Other(format!(
                    "PostponedReceipt {{ receipt_hash: {receipt_hash:?} }}"
                ))
            }
            "not_writable_to_disk" => Self::Other("NotWritableToDisk".into()),
            "initial_state" => Self::Other("InitialState".into()),
            "updated_delayed_receipts" => Self::Other("UpdatedDelayedReceipts".into()),
            "validator_accounts_update" => Self::Other("ValidatorAccountsUpdate".into()),
            "migration" => Self::Other("Migration".into()),
            "resharding_v2" => Self::Other("ReshardingV2".into()),
            "bandwidth_scheduler_state_update" => {
                Self::Other("BandwidthSchedulerStateUpdate".into())
            }
            other => Self::Other(format!("UnknownStateChangeCause({other})")),
        })
    }
}

#[derive(Clone, Deserialize)]
struct RawStateChange {
    cause: RawStateChangeCause,
    #[serde(flatten)]
    value: RawStateChangeValue,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum RawStateChangeValue {
    DataUpdate {
        change: RawDataUpdate,
    },
    DataDeletion {
        change: RawDataDeletion,
    },
    #[serde(other)]
    Other,
}

#[derive(Clone, Deserialize)]
struct RawDataUpdate {
    account_id: AccountId,
    #[serde(rename = "key_base64")]
    key: StoreKey,
    #[serde(rename = "value_base64")]
    value: StoreValue,
}

#[derive(Clone, Deserialize)]
struct RawDataDeletion {
    account_id: AccountId,
    #[serde(rename = "key_base64")]
    key: StoreKey,
}
