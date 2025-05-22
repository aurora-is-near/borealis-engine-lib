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
                .map(convert_validator_stake_view)
                .collect(),
            chunk_mask,
            gas_price,
            block_ordinal,
            total_supply,
            challenges_result: challenges_result
                .into_iter()
                .map(convert_slashed_validator)
                .collect(),
            last_final_block: CryptoHash(last_final_block.0),
            last_ds_final_block: CryptoHash(last_ds_final_block.0),
            next_bp_hash: CryptoHash(next_bp_hash.0),
            block_merkle_root: CryptoHash(block_merkle_root.0),
            epoch_sync_data_hash: epoch_sync_data_hash.map(|h| CryptoHash(h.0)),
            approvals: approvals
                .into_iter()
                .map(|v| v.map(|s| Box::new(convert_signature(*s))))
                .collect(),
            signature: convert_signature(signature),
            latest_protocol_version,
        }
    }
}

fn convert_validator_stake_view(
    v: near_lake_framework::near_indexer_primitives::views::validator_stake_view::ValidatorStakeView,
) -> near_primitives::views::validator_stake_view::ValidatorStakeView {
    match v {
        near_lake_framework::near_indexer_primitives::views::validator_stake_view::ValidatorStakeView::V1(inner) =>{
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

pub fn convert_public_key(v: near_crypto_crates_io::PublicKey) -> near_crypto::PublicKey {
    let key_data = v.key_data();
    match v {
        near_crypto_crates_io::PublicKey::ED25519(inner) => {
            near_crypto::PublicKey::ED25519(inner.0.into())
        }
        near_crypto_crates_io::PublicKey::SECP256K1(_) => {
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

pub fn convert_signature(v: near_crypto_crates_io::Signature) -> near_crypto::Signature {
    match v {
        near_crypto_crates_io::Signature::ED25519(inner) => near_crypto::Signature::ED25519(inner),
        near_crypto_crates_io::Signature::SECP256K1(inner) => {
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

// transaction: tx.transaction.into(),
impl From<near_lake_framework::near_indexer_primitives::views::SignedTransactionView>
    for SignedTransactionView
{
    fn from(
        value: near_lake_framework::near_indexer_primitives::views::SignedTransactionView,
    ) -> Self {
        Self {
            signer_id: value.signer_id,
            public_key: convert_public_key(value.public_key),
            nonce: value.nonce,
            receiver_id: value.receiver_id,
            actions: value.actions.into_iter().map(convert_action_view).collect(),
            priority_fee: value.priority_fee,
            signature: convert_signature(value.signature),
            hash: CryptoHash(value.hash.0),
        }
    }
}

fn convert_action_view(
    v: near_lake_framework::near_indexer_primitives::views::ActionView,
) -> ActionView {
    match v {
        near_primitives_crates_io::views::ActionView::CreateAccount => ActionView::CreateAccount,
        near_primitives_crates_io::views::ActionView::DeployContract { code } => {
            ActionView::DeployContract { code }
        }
        near_primitives_crates_io::views::ActionView::FunctionCall {
            method_name,
            args,
            gas,
            deposit,
        } => ActionView::FunctionCall {
            method_name,
            args: convert_function_call_args(args),
            gas,
            deposit,
        },
        near_primitives_crates_io::views::ActionView::Transfer { deposit } => {
            ActionView::Transfer { deposit }
        }
        near_primitives_crates_io::views::ActionView::Stake { stake, public_key } => {
            ActionView::Stake {
                stake,
                public_key: convert_public_key(public_key),
            }
        }
        near_primitives_crates_io::views::ActionView::AddKey {
            public_key,
            access_key,
        } => ActionView::AddKey {
            public_key: convert_public_key(public_key),
            access_key: convert_access_key_view(access_key),
        },
        near_primitives_crates_io::views::ActionView::DeleteKey { public_key } => {
            ActionView::DeleteKey {
                public_key: convert_public_key(public_key),
            }
        }
        near_primitives_crates_io::views::ActionView::DeleteAccount { beneficiary_id } => {
            ActionView::DeleteAccount { beneficiary_id }
        }
        near_primitives_crates_io::views::ActionView::Delegate {
            delegate_action,
            signature,
        } => ActionView::Delegate {
            delegate_action: convert_delegate_action(delegate_action),
            signature: convert_signature(signature),
        },
        near_primitives_crates_io::views::ActionView::DeployGlobalContract { code } => {
            ActionView::DeployGlobalContract { code }
        }
        near_primitives_crates_io::views::ActionView::DeployGlobalContractByAccountId { code } => {
            ActionView::DeployGlobalContractByAccountId { code }
        }
        near_primitives_crates_io::views::ActionView::UseGlobalContract { code_hash } => {
            ActionView::UseGlobalContract {
                code_hash: CryptoHash(code_hash.0),
            }
        }
        near_primitives_crates_io::views::ActionView::UseGlobalContractByAccountId {
            account_id,
        } => ActionView::UseGlobalContractByAccountId { account_id },
    }
}

fn convert_function_call_args(
    v: near_primitives_crates_io::types::FunctionArgs,
) -> near_primitives::types::FunctionArgs {
    let inner: Vec<u8> = v.into();
    near_primitives::types::FunctionArgs::from(inner)
}

fn convert_access_key(
    v: near_primitives_crates_io::account::AccessKey,
) -> near_primitives::account::AccessKey {
    near_primitives::account::AccessKey {
        nonce: v.nonce,
        permission: convert_access_key_permission(v.permission),
    }
}

fn convert_delegate_action(
    v: near_primitives_crates_io::action::delegate::DelegateAction,
) -> near_primitives::action::delegate::DelegateAction {
    near_primitives::action::delegate::DelegateAction {
        sender_id: v.sender_id,
        receiver_id: v.receiver_id,
        actions: v
            .actions
            .into_iter()
            .map(convert_non_delegate_action)
            .collect(),
        nonce: v.nonce,
        max_block_height: v.max_block_height,
        public_key: convert_public_key(v.public_key),
    }
}

fn convert_non_delegate_action(
    v: near_primitives_crates_io::action::delegate::NonDelegateAction,
) -> near_primitives::action::delegate::NonDelegateAction {
    // Convert through Action first
    let action_inner_crates_io: near_primitives_crates_io::action::Action =
        near_primitives_crates_io::action::Action::from(v);
    let action_inner = match action_inner_crates_io {
        near_primitives_crates_io::action::Action::CreateAccount(_) => {
            near_primitives::action::Action::CreateAccount(
                near_primitives::action::CreateAccountAction {},
            )
        }
        near_primitives_crates_io::action::Action::DeployContract(deploy_contract_action) => {
            near_primitives::action::Action::DeployContract(
                near_primitives::action::DeployContractAction {
                    code: deploy_contract_action.code,
                },
            )
        }
        near_primitives_crates_io::action::Action::FunctionCall(function_call_action) => {
            near_primitives::action::Action::FunctionCall(Box::new(
                near_primitives::action::FunctionCallAction {
                    method_name: function_call_action.method_name,
                    args: function_call_action.args,
                    gas: function_call_action.gas,
                    deposit: function_call_action.deposit,
                },
            ))
        }
        near_primitives_crates_io::action::Action::Transfer(transfer_action) => {
            near_primitives::action::Action::Transfer(near_primitives::action::TransferAction {
                deposit: transfer_action.deposit,
            })
        }
        near_primitives_crates_io::action::Action::Stake(stake_action) => {
            near_primitives::action::Action::Stake(Box::new(near_primitives::action::StakeAction {
                stake: stake_action.stake,
                public_key: convert_public_key(stake_action.public_key),
            }))
        }
        near_primitives_crates_io::action::Action::AddKey(add_key_action) => {
            near_primitives::action::Action::AddKey(Box::new(
                near_primitives::action::AddKeyAction {
                    public_key: convert_public_key(add_key_action.public_key),
                    access_key: convert_access_key(add_key_action.access_key),
                },
            ))
        }
        near_primitives_crates_io::action::Action::DeleteKey(delete_key_action) => {
            near_primitives::action::Action::DeleteKey(Box::new(
                near_primitives::action::DeleteKeyAction {
                    public_key: convert_public_key(delete_key_action.public_key),
                },
            ))
        }
        near_primitives_crates_io::action::Action::DeleteAccount(delete_account_action) => {
            near_primitives::action::Action::DeleteAccount(
                near_primitives::action::DeleteAccountAction {
                    beneficiary_id: delete_account_action.beneficiary_id,
                },
            )
        }
        near_primitives_crates_io::action::Action::Delegate(signed_delegate_action) => {
            near_primitives::action::Action::Delegate(Box::new(
                near_primitives::action::delegate::SignedDelegateAction {
                    delegate_action: convert_delegate_action(
                        signed_delegate_action.delegate_action,
                    ),
                    signature: convert_signature(signed_delegate_action.signature),
                },
            ))
        }
        near_primitives_crates_io::action::Action::DeployGlobalContract(
            deploy_global_contract_action,
        ) => near_primitives::action::Action::DeployGlobalContract(
            near_primitives::action::DeployGlobalContractAction {
                code: deploy_global_contract_action.code,
                deploy_mode: convert_global_contract_deploy_mode(
                    deploy_global_contract_action.deploy_mode,
                ),
            },
        ),
        near_primitives_crates_io::action::Action::UseGlobalContract(
            use_global_contract_action,
        ) => near_primitives::action::Action::UseGlobalContract(Box::new(
            near_primitives::action::UseGlobalContractAction {
                contract_identifier: convert_global_contract_identifier(
                    use_global_contract_action.contract_identifier,
                ),
            },
        )),
    };
    near_primitives::action::delegate::NonDelegateAction::try_from(action_inner)
        .expect("Failed to convert Action to NonDelegateAction")
}

fn convert_access_key_view(
    v: near_primitives_crates_io::views::AccessKeyView,
) -> near_primitives::views::AccessKeyView {
    near_primitives::views::AccessKeyView {
        nonce: v.nonce,
        permission: {
            let inner: near_primitives_core_crates_io::account::AccessKeyPermission =
                v.permission.into();
            convert_access_key_permission(inner).into()
        },
    }
}

fn convert_access_key_permission(
    v: near_primitives_core_crates_io::account::AccessKeyPermission,
) -> near_primitives::account::AccessKeyPermission {
    match v {
        near_primitives_core_crates_io::account::AccessKeyPermission::FunctionCall(inner) => {
            near_primitives::account::AccessKeyPermission::FunctionCall(
                near_primitives::account::FunctionCallPermission {
                    allowance: inner.allowance,
                    method_names: inner.method_names,
                    receiver_id: inner.receiver_id,
                },
            )
        }
        near_primitives_core_crates_io::account::AccessKeyPermission::FullAccess => {
            near_primitives::account::AccessKeyPermission::FullAccess
        }
    }
}

const fn convert_global_contract_deploy_mode(
    v: near_primitives_crates_io::action::GlobalContractDeployMode,
) -> near_primitives::action::GlobalContractDeployMode {
    match v {
        near_primitives_crates_io::action::GlobalContractDeployMode::CodeHash => {
            near_primitives::action::GlobalContractDeployMode::CodeHash
        }
        near_primitives_crates_io::action::GlobalContractDeployMode::AccountId => {
            near_primitives::action::GlobalContractDeployMode::AccountId
        }
    }
}

impl From<near_lake_framework::near_indexer_primitives::views::ReceiptView> for ReceiptView {
    fn from(value: near_lake_framework::near_indexer_primitives::views::ReceiptView) -> Self {
        Self {
            predecessor_id: value.predecessor_id,
            receiver_id: value.receiver_id,
            receipt_id: CryptoHash(value.receipt_id.0),
            receipt: convert_receipt_enum_view(value.receipt),
            priority: value.priority,
        }
    }
}

pub fn convert_receipt_enum_view(
    v: near_lake_framework::near_indexer_primitives::views::ReceiptEnumView,
) -> views::ReceiptEnumView {
    match v {
        near_lake_framework::near_indexer_primitives::views::ReceiptEnumView::Action {
            signer_id,
            signer_public_key,
            gas_price,
            output_data_receivers,
            input_data_ids,
            actions,
            is_promise_yield } =>{
            views::ReceiptEnumView::Action {
                signer_id,
                signer_public_key: convert_public_key(signer_public_key),
                gas_price,
                output_data_receivers: output_data_receivers.into_iter().map(convert_data_receiver_view).collect(),
                input_data_ids: input_data_ids.into_iter().map(|v| CryptoHash(v.0)).collect(),                actions: actions.into_iter().map(convert_action_view).collect(),
                is_promise_yield
            }
        }
        near_lake_framework::near_indexer_primitives::views::ReceiptEnumView::Data { data_id, data, is_promise_resume } =>{
            views::ReceiptEnumView::Data {
                data_id: CryptoHash(data_id.0), data, is_promise_resume
            }
        }
        near_lake_framework::near_indexer_primitives::views::ReceiptEnumView::GlobalContractDistribution {
            id,
            target_shard,
            already_delivered_shards,code
        } =>{
            views::ReceiptEnumView::GlobalContractDistribution {
                id: convert_global_contract_identifier(id),
                target_shard: convert_shard_id(target_shard),
                already_delivered_shards: already_delivered_shards.into_iter().map(convert_shard_id).collect(),
                code
            }
        }
    }
}

pub fn convert_data_receiver_view(
    v: near_lake_framework::near_indexer_primitives::views::DataReceiverView,
) -> views::DataReceiverView {
    views::DataReceiverView {
        data_id: CryptoHash(v.data_id.0),
        receiver_id: v.receiver_id,
    }
}

pub fn convert_global_contract_identifier(
    v: near_primitives_crates_io::action::GlobalContractIdentifier,
) -> near_primitives::action::GlobalContractIdentifier {
    match v {
        near_primitives_crates_io::action::GlobalContractIdentifier::CodeHash(inner) => {
            near_primitives::action::GlobalContractIdentifier::CodeHash(CryptoHash(inner.0))
        }
        near_primitives_crates_io::action::GlobalContractIdentifier::AccountId(inner) => {
            near_primitives::action::GlobalContractIdentifier::AccountId(inner)
        }
    }
}

pub fn convert_shard_id(v: near_primitives_core_crates_io::types::ShardId) -> ShardId {
    ShardId::from(<near_primitives_core_crates_io::types::ShardId as Into<
        u64,
    >>::into(v))
    // near_primitives::types::ShardId::from_le_bytes(src.shard_id.to_le_bytes())
}

//
// Shards conversions
//

// header: chunk.header.into(),
impl From<near_lake_framework::near_indexer_primitives::views::ChunkHeaderView>
    for ChunkHeaderView
{
    fn from(src: near_lake_framework::near_indexer_primitives::views::ChunkHeaderView) -> Self {
        Self {
            chunk_hash: near_primitives::hash::CryptoHash(src.chunk_hash.0),
            prev_block_hash: near_primitives::hash::CryptoHash(src.prev_block_hash.0),
            outcome_root: near_primitives::hash::CryptoHash(src.outcome_root.0),
            prev_state_root: near_primitives::hash::CryptoHash(src.prev_state_root.0),
            encoded_merkle_root: near_primitives::hash::CryptoHash(src.encoded_merkle_root.0),
            encoded_length: src.encoded_length,
            height_created: src.height_created,
            height_included: src.height_included,
            shard_id: convert_shard_id(src.shard_id),
            gas_used: src.gas_used,
            gas_limit: src.gas_limit,
            validator_reward: src.validator_reward,
            balance_burnt: src.balance_burnt,
            outgoing_receipts_root: near_primitives::hash::CryptoHash(src.outgoing_receipts_root.0),
            tx_root: near_primitives::hash::CryptoHash(src.tx_root.0),
            validator_proposals: src
                .validator_proposals
                .into_iter()
                .map(convert_validator_stake_view)
                .collect(),
            signature: convert_signature(src.signature),
        }
    }
}

// outcome: tx.outcome.into(),
impl From<near_lake_framework::near_indexer_primitives::IndexerExecutionOutcomeWithOptionalReceipt>
    for ExecutionOutcomeWithOptionalReceipt
{
    fn from(
        src: near_lake_framework::near_indexer_primitives::IndexerExecutionOutcomeWithOptionalReceipt,
    ) -> Self {
        Self {
            execution_outcome: convert_execution_outcome_with_id_view(src.execution_outcome),
            receipt: src.receipt.map(Into::into),
        }
    }
}

// ExecutionOutcomeWithIdView
pub fn convert_execution_outcome_with_id_view(
    v: near_lake_framework::near_indexer_primitives::views::ExecutionOutcomeWithIdView,
) -> views::ExecutionOutcomeWithIdView {
    views::ExecutionOutcomeWithIdView {
        proof: convert_proof(v.proof),
        block_hash: CryptoHash(v.block_hash.0),
        id: CryptoHash(v.id.0),
        outcome: convert_execution_outcome_view(v.outcome),
    }
}

pub fn convert_proof(
    v: near_lake_framework::near_indexer_primitives::near_primitives::merkle::MerklePath,
) -> near_primitives::merkle::MerklePath {
    v.into_iter().map(convert_merkle_path_item).collect()
}

pub const fn convert_merkle_path_item(
    v: near_lake_framework::near_indexer_primitives::near_primitives::merkle::MerklePathItem,
) -> near_primitives::merkle::MerklePathItem {
    near_primitives::merkle::MerklePathItem {
        hash: CryptoHash(v.hash.0),
        direction: match v.direction {
            near_lake_framework::near_indexer_primitives::near_primitives::merkle::Direction::Left =>
                near_primitives::merkle::Direction::Left,
            near_lake_framework::near_indexer_primitives::near_primitives::merkle::Direction::Right =>
                near_primitives::merkle::Direction::Right,
        },
    }
}

// ExecutionOutcomeView
pub fn convert_execution_outcome_view(
    v: near_lake_framework::near_indexer_primitives::views::ExecutionOutcomeView,
) -> views::ExecutionOutcomeView {
    views::ExecutionOutcomeView {
        logs: v.logs,
        receipt_ids: v.receipt_ids.into_iter().map(|v| CryptoHash(v.0)).collect(),
        gas_burnt: v.gas_burnt,
        tokens_burnt: v.tokens_burnt,
        executor_id: v.executor_id,
        status: convert_execution_status_view(v.status),
        metadata: convert_execution_metadata_view(v.metadata),
    }
}

pub fn convert_execution_status_view(
    v: near_lake_framework::near_indexer_primitives::views::ExecutionStatusView,
) -> views::ExecutionStatusView {
    match v {
        near_lake_framework::near_indexer_primitives::views::ExecutionStatusView::Unknown =>{
            views::ExecutionStatusView::Unknown
        }
        near_lake_framework::near_indexer_primitives::views::ExecutionStatusView::Failure(tx_execution_error) =>{
            views::ExecutionStatusView::Failure(convert_tx_execution_error(tx_execution_error))
        }
        near_lake_framework::near_indexer_primitives::views::ExecutionStatusView::SuccessValue(items) =>{
            views::ExecutionStatusView::SuccessValue(items)
        }
        near_lake_framework::near_indexer_primitives::views::ExecutionStatusView::SuccessReceiptId(crypto_hash) =>{
            views::ExecutionStatusView::SuccessReceiptId(CryptoHash(crypto_hash.0))
        }
    }
}

// ExecutionMetadataView
pub fn convert_execution_metadata_view(
    v: near_lake_framework::near_indexer_primitives::views::ExecutionMetadataView,
) -> views::ExecutionMetadataView {
    views::ExecutionMetadataView {
        version: v.version,
        gas_profile: v
            .gas_profile
            .map(|v| v.into_iter().map(convert_cost_gas_used).collect()),
    }
}

pub fn convert_cost_gas_used(
    v: near_lake_framework::near_indexer_primitives::views::CostGasUsed,
) -> views::CostGasUsed {
    views::CostGasUsed {
        cost_category: v.cost_category,
        cost: v.cost,
        gas_used: v.gas_used,
    }
}

pub fn convert_tx_execution_error(
    v: near_primitives_crates_io::errors::TxExecutionError,
) -> near_primitives::errors::TxExecutionError {
    match v {
        near_primitives_crates_io::errors::TxExecutionError::ActionError(inner) => {
            near_primitives::errors::TxExecutionError::ActionError(convert_action_error(inner))
        }
        near_primitives_crates_io::errors::TxExecutionError::InvalidTxError(inner) => {
            near_primitives::errors::TxExecutionError::InvalidTxError(convert_invalid_tx_error(
                inner,
            ))
        }
    }
}

fn convert_action_error(
    err: near_lake_framework::near_indexer_primitives::near_primitives::errors::ActionError,
) -> near_primitives::errors::ActionError {
    use near_lake_framework::near_indexer_primitives::near_primitives::errors::ActionErrorKind as LakeKind;
    use near_primitives::errors::{ActionError, ActionErrorKind};

    let kind: ActionErrorKind = match err.kind {
        LakeKind::AccountAlreadyExists { account_id } => {
            ActionErrorKind::AccountAlreadyExists { account_id }
        }
        LakeKind::AccountDoesNotExist { account_id } => {
            ActionErrorKind::AccountDoesNotExist { account_id }
        }
        LakeKind::CreateAccountOnlyByRegistrar {
            account_id,
            registrar_account_id,
            predecessor_id,
        } => ActionErrorKind::CreateAccountOnlyByRegistrar {
            account_id,
            registrar_account_id,
            predecessor_id,
        },
        LakeKind::CreateAccountNotAllowed {
            account_id,
            predecessor_id,
        } => ActionErrorKind::CreateAccountNotAllowed {
            account_id,
            predecessor_id,
        },
        LakeKind::ActorNoPermission {
            account_id,
            actor_id,
        } => ActionErrorKind::ActorNoPermission {
            account_id,
            actor_id,
        },
        LakeKind::DeleteKeyDoesNotExist {
            account_id,
            public_key,
        } => ActionErrorKind::DeleteKeyDoesNotExist {
            account_id,
            public_key: Box::new(convert_public_key(*public_key)),
        },
        LakeKind::AddKeyAlreadyExists {
            account_id,
            public_key,
        } => ActionErrorKind::AddKeyAlreadyExists {
            account_id,
            public_key: Box::new(convert_public_key(*public_key)),
        },
        LakeKind::DeleteAccountStaking { account_id } => {
            ActionErrorKind::DeleteAccountStaking { account_id }
        }
        LakeKind::LackBalanceForState { account_id, amount } => {
            ActionErrorKind::LackBalanceForState { account_id, amount }
        }
        LakeKind::TriesToUnstake { account_id } => ActionErrorKind::TriesToUnstake { account_id },
        LakeKind::TriesToStake {
            account_id,
            stake,
            locked,
            balance,
        } => ActionErrorKind::TriesToStake {
            account_id,
            stake,
            locked,
            balance,
        },
        LakeKind::InsufficientStake {
            account_id,
            stake,
            minimum_stake,
        } => ActionErrorKind::InsufficientStake {
            account_id,
            stake,
            minimum_stake,
        },
        LakeKind::FunctionCallError(e) => {
            ActionErrorKind::FunctionCallError(convert_function_call_error(e))
        }
        LakeKind::NewReceiptValidationError(e) => {
            ActionErrorKind::NewReceiptValidationError(convert_new_receipt_validation_error(e))
        }
        LakeKind::OnlyImplicitAccountCreationAllowed { account_id } => {
            ActionErrorKind::OnlyImplicitAccountCreationAllowed { account_id }
        }
        LakeKind::DeleteAccountWithLargeState { account_id } => {
            ActionErrorKind::DeleteAccountWithLargeState { account_id }
        }
        LakeKind::DelegateActionInvalidSignature => ActionErrorKind::DelegateActionInvalidSignature,
        LakeKind::DelegateActionSenderDoesNotMatchTxReceiver {
            sender_id,
            receiver_id,
        } => ActionErrorKind::DelegateActionSenderDoesNotMatchTxReceiver {
            sender_id,
            receiver_id,
        },
        LakeKind::DelegateActionExpired => ActionErrorKind::DelegateActionExpired,
        LakeKind::DelegateActionAccessKeyError(e) => {
            ActionErrorKind::DelegateActionAccessKeyError(convert_invalid_access_key_error(e))
        }
        LakeKind::DelegateActionInvalidNonce {
            delegate_nonce,
            ak_nonce,
        } => ActionErrorKind::DelegateActionInvalidNonce {
            delegate_nonce,
            ak_nonce,
        },
        LakeKind::DelegateActionNonceTooLarge {
            delegate_nonce,
            upper_bound,
        } => ActionErrorKind::DelegateActionNonceTooLarge {
            delegate_nonce,
            upper_bound,
        },
        LakeKind::GlobalContractDoesNotExist { identifier } => {
            ActionErrorKind::GlobalContractDoesNotExist {
                identifier: convert_global_contract_identifier(identifier),
            }
        }
    };
    ActionError {
        index: err.index,
        kind,
    }
}

fn convert_invalid_tx_error(
    v: near_primitives_crates_io::errors::InvalidTxError,
) -> near_primitives::errors::InvalidTxError {
    match v {
        near_primitives_crates_io::errors::InvalidTxError::InvalidAccessKeyError(
            invalid_access_key_error,
        ) => near_primitives::errors::InvalidTxError::InvalidAccessKeyError(
            convert_invalid_access_key_error(invalid_access_key_error),
        ),
        near_primitives_crates_io::errors::InvalidTxError::InvalidSignerId { signer_id } => {
            near_primitives::errors::InvalidTxError::InvalidSignerId { signer_id }
        }
        near_primitives_crates_io::errors::InvalidTxError::SignerDoesNotExist { signer_id } => {
            near_primitives::errors::InvalidTxError::SignerDoesNotExist { signer_id }
        }
        near_primitives_crates_io::errors::InvalidTxError::InvalidNonce { tx_nonce, ak_nonce } => {
            near_primitives::errors::InvalidTxError::InvalidNonce { tx_nonce, ak_nonce }
        }
        near_primitives_crates_io::errors::InvalidTxError::NonceTooLarge {
            tx_nonce,
            upper_bound,
        } => near_primitives::errors::InvalidTxError::NonceTooLarge {
            tx_nonce,
            upper_bound,
        },
        near_primitives_crates_io::errors::InvalidTxError::InvalidReceiverId { receiver_id } => {
            near_primitives::errors::InvalidTxError::InvalidReceiverId { receiver_id }
        }
        near_primitives_crates_io::errors::InvalidTxError::InvalidSignature => {
            near_primitives::errors::InvalidTxError::InvalidSignature
        }
        near_primitives_crates_io::errors::InvalidTxError::NotEnoughBalance {
            signer_id,
            balance,
            cost,
        } => near_primitives::errors::InvalidTxError::NotEnoughBalance {
            signer_id,
            balance,
            cost,
        },
        near_primitives_crates_io::errors::InvalidTxError::LackBalanceForState {
            signer_id,
            amount,
        } => near_primitives::errors::InvalidTxError::LackBalanceForState { signer_id, amount },
        near_primitives_crates_io::errors::InvalidTxError::CostOverflow => {
            near_primitives::errors::InvalidTxError::CostOverflow
        }
        near_primitives_crates_io::errors::InvalidTxError::InvalidChain => {
            near_primitives::errors::InvalidTxError::InvalidChain
        }
        near_primitives_crates_io::errors::InvalidTxError::Expired => {
            near_primitives::errors::InvalidTxError::Expired
        }
        near_primitives_crates_io::errors::InvalidTxError::ActionsValidation(
            actions_validation_error,
        ) => near_primitives::errors::InvalidTxError::ActionsValidation(
            convert_actions_validation_error(actions_validation_error),
        ),
        near_primitives_crates_io::errors::InvalidTxError::TransactionSizeExceeded {
            size,
            limit,
        } => near_primitives::errors::InvalidTxError::TransactionSizeExceeded { size, limit },
        near_primitives_crates_io::errors::InvalidTxError::InvalidTransactionVersion => {
            near_primitives::errors::InvalidTxError::InvalidTransactionVersion
        }
        near_primitives_crates_io::errors::InvalidTxError::StorageError(storage_error) => {
            near_primitives::errors::InvalidTxError::StorageError(convert_storage_error(
                storage_error,
            ))
        }
        near_primitives_crates_io::errors::InvalidTxError::ShardCongested {
            shard_id,
            congestion_level,
        } => near_primitives::errors::InvalidTxError::ShardCongested {
            shard_id,
            congestion_level,
        },
        near_primitives_crates_io::errors::InvalidTxError::ShardStuck {
            shard_id,
            missed_chunks,
        } => near_primitives::errors::InvalidTxError::ShardStuck {
            shard_id,
            missed_chunks,
        },
    }
}

fn convert_function_call_error(
    v: near_primitives_crates_io::errors::FunctionCallError,
) -> near_primitives::errors::FunctionCallError {
    match v {
        near_primitives_crates_io::errors::FunctionCallError::CompilationError(
            compilation_error,
        ) => near_primitives::errors::FunctionCallError::CompilationError(
            convert_compilation_error(compilation_error),
        ),
        near_primitives_crates_io::errors::FunctionCallError::LinkError { msg } => {
            near_primitives::errors::FunctionCallError::LinkError { msg }
        }
        near_primitives_crates_io::errors::FunctionCallError::MethodResolveError(
            method_resolve_error,
        ) => near_primitives::errors::FunctionCallError::MethodResolveError(
            convert_method_resolve_error(method_resolve_error),
        ),
        near_primitives_crates_io::errors::FunctionCallError::WasmTrap(wasm_trap) => {
            near_primitives::errors::FunctionCallError::WasmTrap(convert_wasm_trap(wasm_trap))
        }
        near_primitives_crates_io::errors::FunctionCallError::WasmUnknownError => {
            near_primitives::errors::FunctionCallError::WasmUnknownError
        }
        near_primitives_crates_io::errors::FunctionCallError::HostError(host_error) => {
            near_primitives::errors::FunctionCallError::HostError(convert_host_error(host_error))
        }
        near_primitives_crates_io::errors::FunctionCallError::_EVMError => {
            near_primitives::errors::FunctionCallError::_EVMError
        }
        near_primitives_crates_io::errors::FunctionCallError::ExecutionError(s) => {
            near_primitives::errors::FunctionCallError::ExecutionError(s)
        }
    }
}

fn convert_compilation_error(
    v: near_primitives_crates_io::errors::CompilationError,
) -> near_primitives::errors::CompilationError {
    match v {
        near_primitives_crates_io::errors::CompilationError::CodeDoesNotExist { account_id } => {
            near_primitives::errors::CompilationError::CodeDoesNotExist { account_id }
        }
        near_primitives_crates_io::errors::CompilationError::PrepareError(prepare_error) => {
            near_primitives::errors::CompilationError::PrepareError(convert_prepare_error(
                prepare_error,
            ))
        }
        near_primitives_crates_io::errors::CompilationError::WasmerCompileError { msg } => {
            near_primitives::errors::CompilationError::WasmerCompileError { msg }
        }
    }
}

const fn convert_method_resolve_error(
    v: near_primitives_crates_io::errors::MethodResolveError,
) -> near_primitives::errors::MethodResolveError {
    match v {
        near_primitives_crates_io::errors::MethodResolveError::MethodEmptyName => {
            near_primitives::errors::MethodResolveError::MethodEmptyName
        }
        near_primitives_crates_io::errors::MethodResolveError::MethodNotFound => {
            near_primitives::errors::MethodResolveError::MethodNotFound
        }
        near_primitives_crates_io::errors::MethodResolveError::MethodInvalidSignature => {
            near_primitives::errors::MethodResolveError::MethodInvalidSignature
        }
    }
}

const fn convert_prepare_error(
    v: near_primitives_crates_io::errors::PrepareError,
) -> near_primitives::errors::PrepareError {
    match v {
        near_primitives_crates_io::errors::PrepareError::Serialization => {
            near_primitives::errors::PrepareError::Serialization
        }
        near_primitives_crates_io::errors::PrepareError::Deserialization => {
            near_primitives::errors::PrepareError::Deserialization
        }
        near_primitives_crates_io::errors::PrepareError::InternalMemoryDeclared => {
            near_primitives::errors::PrepareError::InternalMemoryDeclared
        }
        near_primitives_crates_io::errors::PrepareError::GasInstrumentation => {
            near_primitives::errors::PrepareError::GasInstrumentation
        }
        near_primitives_crates_io::errors::PrepareError::StackHeightInstrumentation => {
            near_primitives::errors::PrepareError::StackHeightInstrumentation
        }
        near_primitives_crates_io::errors::PrepareError::Instantiate => {
            near_primitives::errors::PrepareError::Instantiate
        }
        near_primitives_crates_io::errors::PrepareError::Memory => {
            near_primitives::errors::PrepareError::Memory
        }
        near_primitives_crates_io::errors::PrepareError::TooManyFunctions => {
            near_primitives::errors::PrepareError::TooManyFunctions
        }
        near_primitives_crates_io::errors::PrepareError::TooManyLocals => {
            near_primitives::errors::PrepareError::TooManyLocals
        }
    }
}

const fn convert_wasm_trap(
    v: near_primitives_crates_io::errors::WasmTrap,
) -> near_primitives::errors::WasmTrap {
    match v {
        near_primitives_crates_io::errors::WasmTrap::Unreachable => {
            near_primitives::errors::WasmTrap::Unreachable
        }
        near_primitives_crates_io::errors::WasmTrap::IncorrectCallIndirectSignature => {
            near_primitives::errors::WasmTrap::IncorrectCallIndirectSignature
        }
        near_primitives_crates_io::errors::WasmTrap::MemoryOutOfBounds => {
            near_primitives::errors::WasmTrap::MemoryOutOfBounds
        }
        near_primitives_crates_io::errors::WasmTrap::CallIndirectOOB => {
            near_primitives::errors::WasmTrap::CallIndirectOOB
        }
        near_primitives_crates_io::errors::WasmTrap::IllegalArithmetic => {
            near_primitives::errors::WasmTrap::IllegalArithmetic
        }
        near_primitives_crates_io::errors::WasmTrap::MisalignedAtomicAccess => {
            near_primitives::errors::WasmTrap::MisalignedAtomicAccess
        }
        near_primitives_crates_io::errors::WasmTrap::IndirectCallToNull => {
            near_primitives::errors::WasmTrap::IndirectCallToNull
        }
        near_primitives_crates_io::errors::WasmTrap::StackOverflow => {
            near_primitives::errors::WasmTrap::StackOverflow
        }
        near_primitives_crates_io::errors::WasmTrap::GenericTrap => {
            near_primitives::errors::WasmTrap::GenericTrap
        }
    }
}

fn convert_host_error(
    v: near_primitives_crates_io::errors::HostError,
) -> near_primitives::errors::HostError {
    match v {
        near_primitives_crates_io::errors::HostError::BadUTF16 => {
            near_primitives::errors::HostError::BadUTF16
        }
        near_primitives_crates_io::errors::HostError::BadUTF8 => {
            near_primitives::errors::HostError::BadUTF8
        }
        near_primitives_crates_io::errors::HostError::GasExceeded => {
            near_primitives::errors::HostError::GasExceeded
        }
        near_primitives_crates_io::errors::HostError::GasLimitExceeded => {
            near_primitives::errors::HostError::GasLimitExceeded
        }
        near_primitives_crates_io::errors::HostError::BalanceExceeded => {
            near_primitives::errors::HostError::BalanceExceeded
        }
        near_primitives_crates_io::errors::HostError::EmptyMethodName => {
            near_primitives::errors::HostError::EmptyMethodName
        }
        near_primitives_crates_io::errors::HostError::GuestPanic { panic_msg } => {
            near_primitives::errors::HostError::GuestPanic { panic_msg }
        }
        near_primitives_crates_io::errors::HostError::IntegerOverflow => {
            near_primitives::errors::HostError::IntegerOverflow
        }
        near_primitives_crates_io::errors::HostError::InvalidPromiseIndex { promise_idx } => {
            near_primitives::errors::HostError::InvalidPromiseIndex { promise_idx }
        }
        near_primitives_crates_io::errors::HostError::CannotAppendActionToJointPromise => {
            near_primitives::errors::HostError::CannotAppendActionToJointPromise
        }
        near_primitives_crates_io::errors::HostError::CannotReturnJointPromise => {
            near_primitives::errors::HostError::CannotReturnJointPromise
        }
        near_primitives_crates_io::errors::HostError::InvalidPromiseResultIndex { result_idx } => {
            near_primitives::errors::HostError::InvalidPromiseResultIndex { result_idx }
        }
        near_primitives_crates_io::errors::HostError::InvalidRegisterId { register_id } => {
            near_primitives::errors::HostError::InvalidRegisterId { register_id }
        }
        near_primitives_crates_io::errors::HostError::IteratorWasInvalidated { iterator_index } => {
            near_primitives::errors::HostError::IteratorWasInvalidated { iterator_index }
        }
        near_primitives_crates_io::errors::HostError::MemoryAccessViolation => {
            near_primitives::errors::HostError::MemoryAccessViolation
        }
        near_primitives_crates_io::errors::HostError::InvalidReceiptIndex { receipt_index } => {
            near_primitives::errors::HostError::InvalidReceiptIndex { receipt_index }
        }
        near_primitives_crates_io::errors::HostError::InvalidIteratorIndex { iterator_index } => {
            near_primitives::errors::HostError::InvalidIteratorIndex { iterator_index }
        }
        near_primitives_crates_io::errors::HostError::InvalidAccountId => {
            near_primitives::errors::HostError::InvalidAccountId
        }
        near_primitives_crates_io::errors::HostError::InvalidMethodName => {
            near_primitives::errors::HostError::InvalidMethodName
        }
        near_primitives_crates_io::errors::HostError::InvalidPublicKey => {
            near_primitives::errors::HostError::InvalidPublicKey
        }
        near_primitives_crates_io::errors::HostError::ProhibitedInView { method_name } => {
            near_primitives::errors::HostError::ProhibitedInView { method_name }
        }
        near_primitives_crates_io::errors::HostError::NumberOfLogsExceeded { limit } => {
            near_primitives::errors::HostError::NumberOfLogsExceeded { limit }
        }
        near_primitives_crates_io::errors::HostError::KeyLengthExceeded { length, limit } => {
            near_primitives::errors::HostError::KeyLengthExceeded { length, limit }
        }
        near_primitives_crates_io::errors::HostError::ValueLengthExceeded { length, limit } => {
            near_primitives::errors::HostError::ValueLengthExceeded { length, limit }
        }
        near_primitives_crates_io::errors::HostError::TotalLogLengthExceeded { length, limit } => {
            near_primitives::errors::HostError::TotalLogLengthExceeded { length, limit }
        }
        near_primitives_crates_io::errors::HostError::NumberPromisesExceeded {
            number_of_promises,
            limit,
        } => near_primitives::errors::HostError::NumberPromisesExceeded {
            number_of_promises,
            limit,
        },
        near_primitives_crates_io::errors::HostError::NumberInputDataDependenciesExceeded {
            number_of_input_data_dependencies,
            limit,
        } => near_primitives::errors::HostError::NumberInputDataDependenciesExceeded {
            number_of_input_data_dependencies,
            limit,
        },
        near_primitives_crates_io::errors::HostError::ReturnedValueLengthExceeded {
            length,
            limit,
        } => near_primitives::errors::HostError::ReturnedValueLengthExceeded { length, limit },
        near_primitives_crates_io::errors::HostError::ContractSizeExceeded { size, limit } => {
            near_primitives::errors::HostError::ContractSizeExceeded { size, limit }
        }
        near_primitives_crates_io::errors::HostError::Deprecated { method_name } => {
            near_primitives::errors::HostError::Deprecated { method_name }
        }
        near_primitives_crates_io::errors::HostError::ECRecoverError { msg } => {
            near_primitives::errors::HostError::ECRecoverError { msg }
        }
        near_primitives_crates_io::errors::HostError::AltBn128InvalidInput { msg } => {
            near_primitives::errors::HostError::AltBn128InvalidInput { msg }
        }
        near_primitives_crates_io::errors::HostError::Ed25519VerifyInvalidInput { msg } => {
            near_primitives::errors::HostError::Ed25519VerifyInvalidInput { msg }
        }
    }
}

pub fn convert_new_receipt_validation_error(
    v: near_primitives_crates_io::errors::ReceiptValidationError,
) -> near_primitives::errors::ReceiptValidationError {
    match v {
        near_primitives_crates_io::errors::ReceiptValidationError::InvalidPredecessorId { account_id } =>{
            near_primitives::errors::ReceiptValidationError::InvalidPredecessorId { account_id }
        }
        near_primitives_crates_io::errors::ReceiptValidationError::InvalidReceiverId { account_id } =>{
            near_primitives::errors::ReceiptValidationError::InvalidReceiverId { account_id }
        }
        near_primitives_crates_io::errors::ReceiptValidationError::InvalidSignerId { account_id } =>{
            near_primitives::errors::ReceiptValidationError::InvalidSignerId { account_id }
        }
        near_primitives_crates_io::errors::ReceiptValidationError::InvalidDataReceiverId { account_id } =>{
            near_primitives::errors::ReceiptValidationError::InvalidDataReceiverId { account_id }
        }
        near_primitives_crates_io::errors::ReceiptValidationError::ReturnedValueLengthExceeded { length, limit } =>{
            near_primitives::errors::ReceiptValidationError::ReturnedValueLengthExceeded { length, limit }
        }
        near_primitives_crates_io::errors::ReceiptValidationError::NumberInputDataDependenciesExceeded { number_of_input_data_dependencies, limit } =>{
            near_primitives::errors::ReceiptValidationError::NumberInputDataDependenciesExceeded { number_of_input_data_dependencies, limit }
        }
        near_primitives_crates_io::errors::ReceiptValidationError::ActionsValidation(e) =>{
            near_primitives::errors::ReceiptValidationError::ActionsValidation(convert_actions_validation_error(e))
        }
        near_primitives_crates_io::errors::ReceiptValidationError::ReceiptSizeExceeded { size, limit } =>{
            near_primitives::errors::ReceiptValidationError::ReceiptSizeExceeded { size, limit }
        }
    }
}

fn convert_actions_validation_error(
    v: near_primitives_crates_io::errors::ActionsValidationError,
) -> near_primitives::errors::ActionsValidationError {
    match v {
        near_primitives_crates_io::errors::ActionsValidationError::DeleteActionMustBeFinal =>
            near_primitives::errors::ActionsValidationError::DeleteActionMustBeFinal,
        near_primitives_crates_io::errors::ActionsValidationError::TotalPrepaidGasExceeded { total_prepaid_gas, limit } =>
            near_primitives::errors::ActionsValidationError::TotalPrepaidGasExceeded { total_prepaid_gas, limit },
        near_primitives_crates_io::errors::ActionsValidationError::TotalNumberOfActionsExceeded { total_number_of_actions, limit } =>
            near_primitives::errors::ActionsValidationError::TotalNumberOfActionsExceeded { total_number_of_actions, limit },
        near_primitives_crates_io::errors::ActionsValidationError::AddKeyMethodNamesNumberOfBytesExceeded { total_number_of_bytes, limit } =>
            near_primitives::errors::ActionsValidationError::AddKeyMethodNamesNumberOfBytesExceeded { total_number_of_bytes, limit },
        near_primitives_crates_io::errors::ActionsValidationError::AddKeyMethodNameLengthExceeded { length, limit } =>
            near_primitives::errors::ActionsValidationError::AddKeyMethodNameLengthExceeded { length, limit },
        near_primitives_crates_io::errors::ActionsValidationError::IntegerOverflow =>
            near_primitives::errors::ActionsValidationError::IntegerOverflow,
        near_primitives_crates_io::errors::ActionsValidationError::InvalidAccountId { account_id } =>
            near_primitives::errors::ActionsValidationError::InvalidAccountId { account_id },
        near_primitives_crates_io::errors::ActionsValidationError::ContractSizeExceeded { size, limit } =>
            near_primitives::errors::ActionsValidationError::ContractSizeExceeded { size, limit },
        near_primitives_crates_io::errors::ActionsValidationError::FunctionCallMethodNameLengthExceeded { length, limit } =>
            near_primitives::errors::ActionsValidationError::FunctionCallMethodNameLengthExceeded { length, limit },
        near_primitives_crates_io::errors::ActionsValidationError::FunctionCallArgumentsLengthExceeded { length, limit } =>
            near_primitives::errors::ActionsValidationError::FunctionCallArgumentsLengthExceeded { length, limit },
        near_primitives_crates_io::errors::ActionsValidationError::UnsuitableStakingKey { public_key } =>
            near_primitives::errors::ActionsValidationError::UnsuitableStakingKey { public_key: Box::new(convert_public_key(*public_key)) },
        near_primitives_crates_io::errors::ActionsValidationError::FunctionCallZeroAttachedGas =>
            near_primitives::errors::ActionsValidationError::FunctionCallZeroAttachedGas,
        near_primitives_crates_io::errors::ActionsValidationError::DelegateActionMustBeOnlyOne =>
            near_primitives::errors::ActionsValidationError::DelegateActionMustBeOnlyOne,
                near_primitives_crates_io::errors::ActionsValidationError::UnsupportedProtocolFeature { protocol_feature, version } =>
            near_primitives::errors::ActionsValidationError::UnsupportedProtocolFeature { protocol_feature, version },
    }
}

fn convert_storage_error(
    v: near_primitives_crates_io::errors::StorageError,
) -> near_primitives::errors::StorageError {
    match v {
        near_primitives_crates_io::errors::StorageError::StorageInternalError => {
            near_primitives::errors::StorageError::StorageInternalError
        }
        near_primitives_crates_io::errors::StorageError::MissingTrieValue(
            missing_trie_value_context,
            crypto_hash,
        ) => near_primitives::errors::StorageError::MissingTrieValue(
            convert_missing_trie_value_context(missing_trie_value_context),
            CryptoHash(crypto_hash.0),
        ),
        near_primitives_crates_io::errors::StorageError::UnexpectedTrieValue => {
            near_primitives::errors::StorageError::UnexpectedTrieValue
        }
        near_primitives_crates_io::errors::StorageError::StorageInconsistentState(s) => {
            near_primitives::errors::StorageError::StorageInconsistentState(s)
        }
        near_primitives_crates_io::errors::StorageError::FlatStorageBlockNotSupported(s) => {
            near_primitives::errors::StorageError::FlatStorageBlockNotSupported(s)
        }
        near_primitives_crates_io::errors::StorageError::MemTrieLoadingError(s) => {
            near_primitives::errors::StorageError::MemTrieLoadingError(s)
        }
        near_primitives_crates_io::errors::StorageError::FlatStorageReshardingAlreadyInProgress => {
            near_primitives::errors::StorageError::FlatStorageReshardingAlreadyInProgress
        }
    }
}

const fn convert_missing_trie_value_context(
    v: near_primitives_crates_io::errors::MissingTrieValueContext,
) -> near_primitives::errors::MissingTrieValueContext {
    match v {
        near_primitives_crates_io::errors::MissingTrieValueContext::TrieIterator => {
            near_primitives::errors::MissingTrieValueContext::TrieIterator
        }
        near_primitives_crates_io::errors::MissingTrieValueContext::TriePrefetchingStorage => {
            near_primitives::errors::MissingTrieValueContext::TriePrefetchingStorage
        }
        near_primitives_crates_io::errors::MissingTrieValueContext::TrieMemoryPartialStorage => {
            near_primitives::errors::MissingTrieValueContext::TrieMemoryPartialStorage
        }
        near_primitives_crates_io::errors::MissingTrieValueContext::TrieStorage => {
            near_primitives::errors::MissingTrieValueContext::TrieStorage
        }
    }
}

fn convert_invalid_access_key_error(
    v: near_primitives_crates_io::errors::InvalidAccessKeyError,
) -> near_primitives::errors::InvalidAccessKeyError {
    match v {
        near_primitives_crates_io::errors::InvalidAccessKeyError::AccessKeyNotFound {
            account_id,
            public_key,
        } => near_primitives::errors::InvalidAccessKeyError::AccessKeyNotFound {
            account_id,
            public_key: Box::new(convert_public_key(*public_key)),
        },
        near_primitives_crates_io::errors::InvalidAccessKeyError::ReceiverMismatch {
            tx_receiver,
            ak_receiver,
        } => near_primitives::errors::InvalidAccessKeyError::ReceiverMismatch {
            tx_receiver,
            ak_receiver,
        },
        near_primitives_crates_io::errors::InvalidAccessKeyError::MethodNameMismatch {
            method_name,
        } => near_primitives::errors::InvalidAccessKeyError::MethodNameMismatch { method_name },
        near_primitives_crates_io::errors::InvalidAccessKeyError::RequiresFullAccess => {
            near_primitives::errors::InvalidAccessKeyError::RequiresFullAccess
        }
        near_primitives_crates_io::errors::InvalidAccessKeyError::NotEnoughAllowance {
            account_id,
            public_key,
            allowance,
            cost,
        } => near_primitives::errors::InvalidAccessKeyError::NotEnoughAllowance {
            account_id,
            public_key: Box::new(convert_public_key(*public_key)),
            allowance,
            cost,
        },
        near_primitives_crates_io::errors::InvalidAccessKeyError::DepositWithFunctionCall => {
            near_primitives::errors::InvalidAccessKeyError::DepositWithFunctionCall
        }
    }
}

//
// From IndexerShard to Shard
//

// impl From<near_lake_framework::near_indexer_primitives::IndexerShard> for Shard {
//     fn from(view: near_lake_framework::near_indexer_primitives::IndexerShard) -> Self {
//         Self {
//             shard_id: convert_shard_id(view.shard_id),
//             chunk: todo!(),
//             receipt_execution_outcomes: todo!(),
//             state_changes: convert_state_changes_view(view.state_changes),
//         }
//     }
// }

pub fn convert_state_changes_view(
    state_changes: near_lake_framework::near_indexer_primitives::views::StateChangesView,
) -> views::StateChangesView {
    state_changes
        .into_iter()
        .map(convert_state_change_with_cause_view)
        .collect()
}

pub fn convert_state_change_with_cause_view(
    state_change: near_lake_framework::near_indexer_primitives::views::StateChangeWithCauseView,
) -> views::StateChangeWithCauseView {
    views::StateChangeWithCauseView {
        cause: convert_state_change_cause_view(state_change.cause),
        value: convert_state_change_value_view(state_change.value),
    }
}

const fn convert_state_change_cause_view(
    cause: near_lake_framework::near_indexer_primitives::views::StateChangeCauseView,
) -> views::StateChangeCauseView {
    match cause {
        near_primitives_crates_io::views::StateChangeCauseView::NotWritableToDisk =>
            views::StateChangeCauseView::NotWritableToDisk,
        near_primitives_crates_io::views::StateChangeCauseView::InitialState =>
            views::StateChangeCauseView::InitialState,
        near_primitives_crates_io::views::StateChangeCauseView::TransactionProcessing { tx_hash } =>
            views::StateChangeCauseView::TransactionProcessing { tx_hash: CryptoHash(tx_hash.0) },
        near_primitives_crates_io::views::StateChangeCauseView::ActionReceiptProcessingStarted { receipt_hash } =>
            views::StateChangeCauseView::ActionReceiptProcessingStarted { receipt_hash: CryptoHash(receipt_hash.0) },
        near_primitives_crates_io::views::StateChangeCauseView::ActionReceiptGasReward { receipt_hash } =>
            views::StateChangeCauseView::ActionReceiptGasReward { receipt_hash: CryptoHash(receipt_hash.0) },
        near_primitives_crates_io::views::StateChangeCauseView::ReceiptProcessing { receipt_hash } =>
            views::StateChangeCauseView::ReceiptProcessing { receipt_hash: CryptoHash(receipt_hash.0) },
        near_primitives_crates_io::views::StateChangeCauseView::PostponedReceipt { receipt_hash } =>
            views::StateChangeCauseView::PostponedReceipt { receipt_hash: CryptoHash(receipt_hash.0) },
        near_primitives_crates_io::views::StateChangeCauseView::UpdatedDelayedReceipts =>
            views::StateChangeCauseView::UpdatedDelayedReceipts,
        near_primitives_crates_io::views::StateChangeCauseView::ValidatorAccountsUpdate =>
            views::StateChangeCauseView::ValidatorAccountsUpdate,
        near_primitives_crates_io::views::StateChangeCauseView::Migration =>
            views::StateChangeCauseView::Migration,
        near_primitives_crates_io::views::StateChangeCauseView::ReshardingV2 =>
            views::StateChangeCauseView::ReshardingV2,
        near_primitives_crates_io::views::StateChangeCauseView::BandwidthSchedulerStateUpdate =>
            views::StateChangeCauseView::BandwidthSchedulerStateUpdate,
    }
}

fn convert_state_change_value_view(
    value: near_lake_framework::near_indexer_primitives::views::StateChangeValueView,
) -> views::StateChangeValueView {
    match value {
        near_primitives_crates_io::views::StateChangeValueView::AccountUpdate {
            account_id,
            account,
        } => views::StateChangeValueView::AccountUpdate {
            account_id,
            account: convert_account_view(account),
        },
        near_primitives_crates_io::views::StateChangeValueView::AccountDeletion { account_id } => {
            views::StateChangeValueView::AccountDeletion { account_id }
        }
        near_primitives_crates_io::views::StateChangeValueView::AccessKeyUpdate {
            account_id,
            public_key,
            access_key,
        } => views::StateChangeValueView::AccessKeyUpdate {
            account_id,
            public_key: convert_public_key(public_key),
            access_key: convert_access_key_view(access_key),
        },
        near_primitives_crates_io::views::StateChangeValueView::AccessKeyDeletion {
            account_id,
            public_key,
        } => views::StateChangeValueView::AccessKeyDeletion {
            account_id,
            public_key: convert_public_key(public_key),
        },
        near_primitives_crates_io::views::StateChangeValueView::DataUpdate {
            account_id,
            key,
            value,
        } => views::StateChangeValueView::DataUpdate {
            account_id,
            key: convert_store_key(key),
            value: convert_store_value(value),
        },
        near_primitives_crates_io::views::StateChangeValueView::DataDeletion {
            account_id,
            key,
        } => views::StateChangeValueView::DataDeletion {
            account_id,
            key: convert_store_key(key),
        },
        near_primitives_crates_io::views::StateChangeValueView::ContractCodeUpdate {
            account_id,
            code,
        } => views::StateChangeValueView::ContractCodeUpdate { account_id, code },
        near_primitives_crates_io::views::StateChangeValueView::ContractCodeDeletion {
            account_id,
        } => views::StateChangeValueView::ContractCodeDeletion { account_id },
    }
}

fn convert_account_view(
    account: near_primitives_crates_io::views::AccountView,
) -> views::AccountView {
    views::AccountView {
        amount: account.amount,
        locked: account.locked,
        code_hash: CryptoHash(account.code_hash.0),
        storage_usage: account.storage_usage,
        storage_paid_at: account.storage_paid_at,
        global_contract_hash: account.global_contract_hash.map(|hash| CryptoHash(hash.0)),
        global_contract_account_id: account.global_contract_account_id,
    }
}

fn convert_store_key(
    key: near_primitives_crates_io::types::StoreKey,
) -> near_primitives::types::StoreKey {
    let key: Vec<u8> = key.into();
    near_primitives::types::StoreKey::from(key)
}

fn convert_store_value(
    value: near_primitives_crates_io::types::StoreValue,
) -> near_primitives::types::StoreValue {
    let value: Vec<u8> = value.into();
    near_primitives::types::StoreValue::from(value)
}
