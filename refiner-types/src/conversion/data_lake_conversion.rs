use std::str::FromStr;

use near_crypto::{ED25519PublicKey, PublicKey, Secp256K1PublicKey, Signature};
use near_primitives::{
    account::{AccessKey, AccessKeyPermission, FunctionCallPermission},
    action::{
        GlobalContractDeployMode, GlobalContractIdentifier,
        delegate::{DelegateAction, NonDelegateAction},
    },
    challenge::SlashedValidator,
    errors::{
        ActionError, ActionsValidationError, CompilationError, FunctionCallError, HostError,
        InvalidAccessKeyError, InvalidTxError, MethodResolveError, MissingTrieValueContext,
        PrepareError, ReceiptValidationError, StorageError, TxExecutionError, WasmTrap,
    },
    hash::CryptoHash,
    types::{FunctionArgs, ShardId, StoreKey, StoreValue},
    views::{
        AccessKeyView, AccountView, ActionView, CostGasUsed, DataReceiverView,
        ExecutionOutcomeView, ExecutionOutcomeWithIdView, ExecutionStatusView, ReceiptEnumView,
        StateChangeCauseView, StateChangeValueView, StateChangeWithCauseView, StateChangesView,
        validator_stake_view::ValidatorStakeView,
    },
};

use crate::{
    Converter,
    near_block::{
        BlockView, ChunkHeaderView, ExecutionOutcomeWithOptionalReceipt, IndexerBlockHeaderView,
        ReceiptView, SignedTransactionView,
    },
};

//
// Base types
//

impl Converter<ShardId> for near_primitives_crates_io::types::ShardId {
    fn convert(self) -> ShardId {
        ShardId::new(self.into())
    }
}

impl Converter<Self> for near_primitives_crates_io::types::AccountId {
    fn convert(self) -> Self {
        Self::from_str(self.as_str()).unwrap()
    }
}

impl Converter<CryptoHash> for near_primitives_crates_io::hash::CryptoHash {
    fn convert(self) -> CryptoHash {
        CryptoHash(self.0)
    }
}

impl Converter<Signature> for near_crypto_crates_io::Signature {
    fn convert(self) -> Signature {
        match self {
            Self::ED25519(s) => Signature::ED25519(s),
            Self::SECP256K1(s) => {
                let s: [u8; 65] = s.into();
                Signature::SECP256K1(near_crypto::Secp256K1Signature::from(s))
            }
        }
    }
}

impl Converter<PublicKey> for near_crypto_crates_io::PublicKey {
    fn convert(self) -> PublicKey {
        match self {
            Self::ED25519(s) => PublicKey::ED25519(ED25519PublicKey(s.0)),
            Self::SECP256K1(s) => {
                PublicKey::SECP256K1(Secp256K1PublicKey::try_from(s.as_ref()).expect("Failed to convert Secp256K1PublicKey from near_crypto_crates_io::PublicKey to near_crypto::PublicKey"))
            }
        }
    }
}

impl Converter<AccessKey> for near_primitives_crates_io::account::AccessKey {
    fn convert(self) -> AccessKey {
        AccessKey {
            nonce: self.nonce,
            permission: self.permission.convert(),
        }
    }
}

impl Converter<AccessKeyPermission>
    for near_primitives_core_crates_io::account::AccessKeyPermission
{
    fn convert(self) -> AccessKeyPermission {
        match self {
            Self::FunctionCall(inner) => {
                AccessKeyPermission::FunctionCall(FunctionCallPermission {
                    allowance: inner.allowance,
                    method_names: inner.method_names,
                    receiver_id: inner.receiver_id,
                })
            }
            Self::FullAccess => AccessKeyPermission::FullAccess,
        }
    }
}

impl Converter<AccessKeyView> for near_primitives_crates_io::views::AccessKeyView {
    fn convert(self) -> AccessKeyView {
        AccessKeyView {
            nonce: self.nonce,
            permission: {
                let inner: near_primitives_core_crates_io::account::AccessKeyPermission =
                    self.permission.into();
                inner.convert().into()
            },
        }
    }
}

impl Converter<GlobalContractDeployMode>
    for near_primitives_crates_io::action::GlobalContractDeployMode
{
    fn convert(self) -> GlobalContractDeployMode {
        match self {
            Self::CodeHash => near_primitives::action::GlobalContractDeployMode::CodeHash,
            Self::AccountId => near_primitives::action::GlobalContractDeployMode::AccountId,
        }
    }
}

impl Converter<GlobalContractIdentifier>
    for near_primitives_crates_io::action::GlobalContractIdentifier
{
    fn convert(self) -> GlobalContractIdentifier {
        match self {
            Self::CodeHash(inner) => {
                near_primitives::action::GlobalContractIdentifier::CodeHash(inner.convert())
            }
            Self::AccountId(inner) => {
                near_primitives::action::GlobalContractIdentifier::AccountId(inner)
            }
        }
    }
}

impl Converter<StoreKey> for near_primitives_crates_io::types::StoreKey {
    fn convert(self) -> StoreKey {
        let key: Vec<u8> = self.into();
        StoreKey::from(key)
    }
}

impl Converter<StoreValue> for near_primitives_crates_io::types::StoreValue {
    fn convert(self) -> StoreValue {
        let value: Vec<u8> = self.into();
        StoreValue::from(value)
    }
}

//
// BlockView
//

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
            epoch_id: epoch_id.convert(),
            next_epoch_id: next_epoch_id.convert(),
            hash: hash.convert(),
            prev_hash: prev_hash.convert(),
            prev_state_root: prev_state_root.convert(),
            chunk_receipts_root: chunk_receipts_root.convert(),
            chunk_headers_root: chunk_headers_root.convert(),
            chunk_tx_root: chunk_tx_root.convert(),
            outcome_root: outcome_root.convert(),
            chunks_included,
            challenges_root: challenges_root.convert(),
            timestamp,
            timestamp_nanosec,
            random_value: random_value.convert(),
            validator_proposals: validator_proposals
                .into_iter()
                .map(Converter::convert)
                .collect(),
            chunk_mask,
            gas_price,
            block_ordinal,
            total_supply,
            challenges_result: challenges_result
                .into_iter()
                .map(Converter::convert)
                .collect(),
            last_final_block: last_final_block.convert(),
            last_ds_final_block: last_ds_final_block.convert(),
            next_bp_hash: next_bp_hash.convert(),
            block_merkle_root: block_merkle_root.convert(),
            epoch_sync_data_hash: epoch_sync_data_hash.map(Converter::convert),
            approvals: approvals
                .into_iter()
                .map(|v| v.map(|s| Box::new(Converter::convert(*s))))
                .collect(),
            signature: signature.convert(),
            latest_protocol_version,
        }
    }
}

impl Converter<ValidatorStakeView> for near_lake_framework::near_indexer_primitives::views::validator_stake_view::ValidatorStakeView {
    fn convert(self) -> ValidatorStakeView {
        match self {
            Self::V1(inner) =>{
                near_primitives::views::validator_stake_view::ValidatorStakeView::V1(
                    near_primitives::views::validator_stake_view::ValidatorStakeViewV1 {
                        account_id: inner.account_id,
                        public_key: inner.public_key.convert(),
                        stake: inner.stake,
                    }
                )
            }
        }
    }
}

impl Converter<SlashedValidator> for near_primitives_crates_io::challenge::SlashedValidator {
    fn convert(self) -> SlashedValidator {
        SlashedValidator {
            account_id: self.account_id,
            is_double_sign: self.is_double_sign,
        }
    }
}

//
// SignedTransactionView
//

impl From<near_lake_framework::near_indexer_primitives::views::SignedTransactionView>
    for SignedTransactionView
{
    fn from(
        value: near_lake_framework::near_indexer_primitives::views::SignedTransactionView,
    ) -> Self {
        Self {
            signer_id: value.signer_id,
            public_key: value.public_key.convert(),
            nonce: value.nonce,
            receiver_id: value.receiver_id,
            actions: value.actions.into_iter().map(Converter::convert).collect(),
            priority_fee: value.priority_fee,
            signature: value.signature.convert(),
            hash: CryptoHash(value.hash.0),
        }
    }
}

//
// ActionView
//

impl Converter<ActionView> for near_lake_framework::near_indexer_primitives::views::ActionView {
    fn convert(self) -> ActionView {
        match self {
            Self::CreateAccount => ActionView::CreateAccount,
            Self::DeployContract { code } => ActionView::DeployContract { code },
            Self::FunctionCall {
                method_name,
                args,
                gas,
                deposit,
            } => ActionView::FunctionCall {
                method_name,
                args: args.convert(),
                gas,
                deposit,
            },
            Self::Transfer { deposit } => ActionView::Transfer { deposit },
            Self::Stake { stake, public_key } => ActionView::Stake {
                stake,
                public_key: public_key.convert(),
            },
            Self::AddKey {
                public_key,
                access_key,
            } => ActionView::AddKey {
                public_key: public_key.convert(),
                access_key: access_key.convert(),
            },
            Self::DeleteKey { public_key } => ActionView::DeleteKey {
                public_key: public_key.convert(),
            },
            Self::DeleteAccount { beneficiary_id } => ActionView::DeleteAccount { beneficiary_id },
            Self::Delegate {
                delegate_action,
                signature,
            } => ActionView::Delegate {
                delegate_action: delegate_action.convert(),
                signature: signature.convert(),
            },
            Self::DeployGlobalContract { code } => ActionView::DeployGlobalContract { code },
            Self::DeployGlobalContractByAccountId { code } => {
                ActionView::DeployGlobalContractByAccountId { code }
            }
            Self::UseGlobalContract { code_hash } => ActionView::UseGlobalContract {
                code_hash: code_hash.convert(),
            },
            Self::UseGlobalContractByAccountId { account_id } => {
                ActionView::UseGlobalContractByAccountId { account_id }
            }
        }
    }
}

impl Converter<FunctionArgs> for near_primitives_crates_io::types::FunctionArgs {
    fn convert(self) -> FunctionArgs {
        let inner: Vec<u8> = self.into();
        FunctionArgs::from(inner)
    }
}

impl Converter<DelegateAction> for near_primitives_crates_io::action::delegate::DelegateAction {
    fn convert(self) -> DelegateAction {
        near_primitives::action::delegate::DelegateAction {
            sender_id: self.sender_id,
            receiver_id: self.receiver_id,
            actions: self.actions.into_iter().map(Converter::convert).collect(),
            nonce: self.nonce,
            max_block_height: self.max_block_height,
            public_key: self.public_key.convert(),
        }
    }
}

impl Converter<NonDelegateAction>
    for near_primitives_crates_io::action::delegate::NonDelegateAction
{
    fn convert(self) -> NonDelegateAction {
        {
            // Convert through Action first
            let action_inner_crates_io: near_primitives_crates_io::action::Action =
                near_primitives_crates_io::action::Action::from(self);
            let action_inner = match action_inner_crates_io {
                near_primitives_crates_io::action::Action::CreateAccount(_) => {
                    near_primitives::action::Action::CreateAccount(
                        near_primitives::action::CreateAccountAction {},
                    )
                }
                near_primitives_crates_io::action::Action::DeployContract(
                    deploy_contract_action,
                ) => near_primitives::action::Action::DeployContract(
                    near_primitives::action::DeployContractAction {
                        code: deploy_contract_action.code,
                    },
                ),
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
                    near_primitives::action::Action::Transfer(
                        near_primitives::action::TransferAction {
                            deposit: transfer_action.deposit,
                        },
                    )
                }
                near_primitives_crates_io::action::Action::Stake(stake_action) => {
                    near_primitives::action::Action::Stake(Box::new(
                        near_primitives::action::StakeAction {
                            stake: stake_action.stake,
                            public_key: stake_action.public_key.convert(),
                        },
                    ))
                }
                near_primitives_crates_io::action::Action::AddKey(add_key_action) => {
                    near_primitives::action::Action::AddKey(Box::new(
                        near_primitives::action::AddKeyAction {
                            public_key: add_key_action.public_key.convert(),
                            access_key: add_key_action.access_key.convert(),
                        },
                    ))
                }
                near_primitives_crates_io::action::Action::DeleteKey(delete_key_action) => {
                    near_primitives::action::Action::DeleteKey(Box::new(
                        near_primitives::action::DeleteKeyAction {
                            public_key: delete_key_action.public_key.convert(),
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
                            delegate_action: signed_delegate_action.delegate_action.convert(),
                            signature: signed_delegate_action.signature.convert(),
                        },
                    ))
                }
                near_primitives_crates_io::action::Action::DeployGlobalContract(
                    deploy_global_contract_action,
                ) => near_primitives::action::Action::DeployGlobalContract(
                    near_primitives::action::DeployGlobalContractAction {
                        code: deploy_global_contract_action.code,
                        deploy_mode: deploy_global_contract_action.deploy_mode.convert(),
                    },
                ),
                near_primitives_crates_io::action::Action::UseGlobalContract(
                    use_global_contract_action,
                ) => near_primitives::action::Action::UseGlobalContract(Box::new(
                    near_primitives::action::UseGlobalContractAction {
                        contract_identifier: use_global_contract_action
                            .contract_identifier
                            .convert(),
                    },
                )),
            };
            near_primitives::action::delegate::NonDelegateAction::try_from(action_inner)
                .expect("Failed to convert Action to NonDelegateAction")
        }
    }
}

//
// ReceiptView
//

impl From<near_lake_framework::near_indexer_primitives::views::ReceiptView> for ReceiptView {
    fn from(value: near_lake_framework::near_indexer_primitives::views::ReceiptView) -> Self {
        Self {
            predecessor_id: value.predecessor_id,
            receiver_id: value.receiver_id,
            receipt_id: value.receipt_id.convert(),
            receipt: value.receipt.convert(),
            priority: value.priority,
        }
    }
}

impl Converter<ReceiptEnumView>
    for near_lake_framework::near_indexer_primitives::views::ReceiptEnumView
{
    fn convert(self) -> ReceiptEnumView {
        match self {
            Self::Action {
                signer_id,
                signer_public_key,
                gas_price,
                output_data_receivers,
                input_data_ids,
                actions,
                is_promise_yield,
            } => ReceiptEnumView::Action {
                signer_id,
                signer_public_key: signer_public_key.convert(),
                gas_price,
                output_data_receivers: output_data_receivers
                    .into_iter()
                    .map(Converter::convert)
                    .collect(),
                input_data_ids: input_data_ids.into_iter().map(Converter::convert).collect(),
                actions: actions.into_iter().map(Converter::convert).collect(),
                is_promise_yield,
            },
            Self::Data {
                data_id,
                data,
                is_promise_resume,
            } => ReceiptEnumView::Data {
                data_id: data_id.convert(),
                data,
                is_promise_resume,
            },
            Self::GlobalContractDistribution {
                id,
                target_shard,
                already_delivered_shards,
                code,
            } => ReceiptEnumView::GlobalContractDistribution {
                id: id.convert(),
                target_shard: target_shard.convert(),
                already_delivered_shards: already_delivered_shards
                    .into_iter()
                    .map(Converter::convert)
                    .collect(),
                code,
            },
        }
    }
}

impl Converter<DataReceiverView>
    for near_lake_framework::near_indexer_primitives::views::DataReceiverView
{
    fn convert(self) -> DataReceiverView {
        DataReceiverView {
            data_id: self.data_id.convert(),
            receiver_id: self.receiver_id,
        }
    }
}

//
// Shards conversions
//

// chunk.header
impl From<near_lake_framework::near_indexer_primitives::views::ChunkHeaderView>
    for ChunkHeaderView
{
    fn from(src: near_lake_framework::near_indexer_primitives::views::ChunkHeaderView) -> Self {
        Self {
            chunk_hash: src.chunk_hash.convert(),
            prev_block_hash: src.prev_block_hash.convert(),
            outcome_root: src.outcome_root.convert(),
            prev_state_root: src.prev_state_root.convert(),
            encoded_merkle_root: src.encoded_merkle_root.convert(),
            encoded_length: src.encoded_length,
            height_created: src.height_created,
            height_included: src.height_included,
            shard_id: src.shard_id.convert(),
            gas_used: src.gas_used,
            gas_limit: src.gas_limit,
            validator_reward: src.validator_reward,
            balance_burnt: src.balance_burnt,
            outgoing_receipts_root: src.outgoing_receipts_root.convert(),
            tx_root: src.tx_root.convert(),
            validator_proposals: src
                .validator_proposals
                .into_iter()
                .map(Converter::convert)
                .collect(),
            signature: src.signature.convert(),
        }
    }
}

// tx.outcome
impl From<near_lake_framework::near_indexer_primitives::IndexerExecutionOutcomeWithOptionalReceipt>
    for ExecutionOutcomeWithOptionalReceipt
{
    fn from(
        src: near_lake_framework::near_indexer_primitives::IndexerExecutionOutcomeWithOptionalReceipt,
    ) -> Self {
        Self {
            execution_outcome: src.execution_outcome.convert(),
            receipt: src.receipt.map(Into::into),
        }
    }
}

impl Converter<ExecutionOutcomeWithIdView>
    for near_lake_framework::near_indexer_primitives::views::ExecutionOutcomeWithIdView
{
    fn convert(self) -> ExecutionOutcomeWithIdView {
        ExecutionOutcomeWithIdView {
            proof: self.proof.into_iter().map(Converter::convert).collect(),
            block_hash: self.block_hash.convert(),
            id: self.id.convert(),
            outcome: self.outcome.convert(),
        }
    }
}

impl Converter<near_primitives::merkle::MerklePathItem>
    for near_lake_framework::near_indexer_primitives::near_primitives::merkle::MerklePathItem
{
    fn convert(self) -> near_primitives::merkle::MerklePathItem {
        near_primitives::merkle::MerklePathItem {
                hash: self.hash.convert(),
                direction: match self.direction {
                    near_lake_framework::near_indexer_primitives::near_primitives::merkle::Direction::Left =>
                        near_primitives::merkle::Direction::Left,
                    near_lake_framework::near_indexer_primitives::near_primitives::merkle::Direction::Right =>
                        near_primitives::merkle::Direction::Right,
                },
            }
    }
}

//
// ExecutionOutcomeView
//

impl Converter<ExecutionOutcomeView>
    for near_lake_framework::near_indexer_primitives::views::ExecutionOutcomeView
{
    fn convert(self) -> ExecutionOutcomeView {
        ExecutionOutcomeView {
            logs: self.logs,
            receipt_ids: self
                .receipt_ids
                .into_iter()
                .map(Converter::convert)
                .collect(),
            gas_burnt: self.gas_burnt,
            tokens_burnt: self.tokens_burnt,
            executor_id: self.executor_id,
            status: self.status.convert(),
            metadata: self.metadata.convert(),
        }
    }
}

//
// ExecutionStatusView
//

impl Converter<ExecutionStatusView>
    for near_lake_framework::near_indexer_primitives::views::ExecutionStatusView
{
    fn convert(self) -> ExecutionStatusView {
        match self {
            Self::Unknown => ExecutionStatusView::Unknown,
            Self::Failure(tx_execution_error) => {
                ExecutionStatusView::Failure(tx_execution_error.convert())
            }
            Self::SuccessValue(items) => ExecutionStatusView::SuccessValue(items),
            Self::SuccessReceiptId(crypto_hash) => {
                ExecutionStatusView::SuccessReceiptId(crypto_hash.convert())
            }
        }
    }
}

impl Converter<near_primitives::views::ExecutionMetadataView>
    for near_lake_framework::near_indexer_primitives::views::ExecutionMetadataView
{
    fn convert(self) -> near_primitives::views::ExecutionMetadataView {
        near_primitives::views::ExecutionMetadataView {
            version: self.version,
            gas_profile: self
                .gas_profile
                .map(|v| v.into_iter().map(Converter::convert).collect()),
        }
    }
}

impl Converter<CostGasUsed> for near_lake_framework::near_indexer_primitives::views::CostGasUsed {
    fn convert(self) -> CostGasUsed {
        CostGasUsed {
            cost_category: self.cost_category,
            cost: self.cost,
            gas_used: self.gas_used,
        }
    }
}

impl Converter<TxExecutionError> for near_primitives_crates_io::errors::TxExecutionError {
    fn convert(self) -> near_primitives::errors::TxExecutionError {
        match self {
            Self::ActionError(err) => {
                near_primitives::errors::TxExecutionError::ActionError(err.convert())
            }
            Self::InvalidTxError(err) => {
                near_primitives::errors::TxExecutionError::InvalidTxError(err.convert())
            }
        }
    }
}

impl Converter<ActionError>
    for near_lake_framework::near_indexer_primitives::near_primitives::errors::ActionError
{
    fn convert(self) -> near_primitives::errors::ActionError {
        {
            use near_lake_framework::near_indexer_primitives::near_primitives::errors::ActionErrorKind as LakeKind;
            use near_primitives::errors::{ActionError, ActionErrorKind};

            let kind: ActionErrorKind = match self.kind {
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
                    public_key: Box::new(public_key.convert()),
                },
                LakeKind::AddKeyAlreadyExists {
                    account_id,
                    public_key,
                } => ActionErrorKind::AddKeyAlreadyExists {
                    account_id,
                    public_key: Box::new(public_key.convert()),
                },
                LakeKind::DeleteAccountStaking { account_id } => {
                    ActionErrorKind::DeleteAccountStaking { account_id }
                }
                LakeKind::LackBalanceForState { account_id, amount } => {
                    ActionErrorKind::LackBalanceForState { account_id, amount }
                }
                LakeKind::TriesToUnstake { account_id } => {
                    ActionErrorKind::TriesToUnstake { account_id }
                }
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
                LakeKind::FunctionCallError(e) => ActionErrorKind::FunctionCallError(e.convert()),
                LakeKind::NewReceiptValidationError(e) => {
                    ActionErrorKind::NewReceiptValidationError(e.convert())
                }
                LakeKind::OnlyImplicitAccountCreationAllowed { account_id } => {
                    ActionErrorKind::OnlyImplicitAccountCreationAllowed { account_id }
                }
                LakeKind::DeleteAccountWithLargeState { account_id } => {
                    ActionErrorKind::DeleteAccountWithLargeState { account_id }
                }
                LakeKind::DelegateActionInvalidSignature => {
                    ActionErrorKind::DelegateActionInvalidSignature
                }
                LakeKind::DelegateActionSenderDoesNotMatchTxReceiver {
                    sender_id,
                    receiver_id,
                } => ActionErrorKind::DelegateActionSenderDoesNotMatchTxReceiver {
                    sender_id,
                    receiver_id,
                },
                LakeKind::DelegateActionExpired => ActionErrorKind::DelegateActionExpired,
                LakeKind::DelegateActionAccessKeyError(e) => {
                    ActionErrorKind::DelegateActionAccessKeyError(e.convert())
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
                        identifier: identifier.convert(),
                    }
                }
            };
            ActionError {
                index: self.index,
                kind,
            }
        }
    }
}

impl Converter<InvalidTxError> for near_primitives_crates_io::errors::InvalidTxError {
    fn convert(self) -> InvalidTxError {
        match self {
            Self::InvalidAccessKeyError(invalid_access_key_error) => {
                near_primitives::errors::InvalidTxError::InvalidAccessKeyError(
                    invalid_access_key_error.convert(),
                )
            }
            Self::InvalidSignerId { signer_id } => {
                near_primitives::errors::InvalidTxError::InvalidSignerId { signer_id }
            }
            Self::SignerDoesNotExist { signer_id } => {
                near_primitives::errors::InvalidTxError::SignerDoesNotExist { signer_id }
            }
            Self::InvalidNonce { tx_nonce, ak_nonce } => {
                near_primitives::errors::InvalidTxError::InvalidNonce { tx_nonce, ak_nonce }
            }
            Self::NonceTooLarge {
                tx_nonce,
                upper_bound,
            } => near_primitives::errors::InvalidTxError::NonceTooLarge {
                tx_nonce,
                upper_bound,
            },
            Self::InvalidReceiverId { receiver_id } => {
                near_primitives::errors::InvalidTxError::InvalidReceiverId { receiver_id }
            }
            Self::InvalidSignature => near_primitives::errors::InvalidTxError::InvalidSignature,
            Self::NotEnoughBalance {
                signer_id,
                balance,
                cost,
            } => near_primitives::errors::InvalidTxError::NotEnoughBalance {
                signer_id,
                balance,
                cost,
            },
            Self::LackBalanceForState { signer_id, amount } => {
                near_primitives::errors::InvalidTxError::LackBalanceForState { signer_id, amount }
            }
            Self::CostOverflow => near_primitives::errors::InvalidTxError::CostOverflow,
            Self::InvalidChain => near_primitives::errors::InvalidTxError::InvalidChain,
            Self::Expired => near_primitives::errors::InvalidTxError::Expired,
            Self::ActionsValidation(actions_validation_error) => {
                near_primitives::errors::InvalidTxError::ActionsValidation(
                    actions_validation_error.convert(),
                )
            }
            Self::TransactionSizeExceeded { size, limit } => {
                near_primitives::errors::InvalidTxError::TransactionSizeExceeded { size, limit }
            }
            Self::InvalidTransactionVersion => {
                near_primitives::errors::InvalidTxError::InvalidTransactionVersion
            }
            Self::StorageError(storage_error) => {
                near_primitives::errors::InvalidTxError::StorageError(storage_error.convert())
            }
            Self::ShardCongested {
                shard_id,
                congestion_level,
            } => near_primitives::errors::InvalidTxError::ShardCongested {
                shard_id,
                congestion_level,
            },
            Self::ShardStuck {
                shard_id,
                missed_chunks,
            } => near_primitives::errors::InvalidTxError::ShardStuck {
                shard_id,
                missed_chunks,
            },
        }
    }
}

impl Converter<FunctionCallError> for near_primitives_crates_io::errors::FunctionCallError {
    fn convert(self) -> FunctionCallError {
        match self {
            Self::CompilationError(compilation_error) => {
                near_primitives::errors::FunctionCallError::CompilationError(
                    compilation_error.convert(),
                )
            }
            Self::LinkError { msg } => {
                near_primitives::errors::FunctionCallError::LinkError { msg }
            }
            Self::MethodResolveError(method_resolve_error) => {
                near_primitives::errors::FunctionCallError::MethodResolveError(
                    method_resolve_error.convert(),
                )
            }
            Self::WasmTrap(wasm_trap) => {
                near_primitives::errors::FunctionCallError::WasmTrap(wasm_trap.convert())
            }
            Self::WasmUnknownError => near_primitives::errors::FunctionCallError::WasmUnknownError,
            Self::HostError(host_error) => {
                near_primitives::errors::FunctionCallError::HostError(host_error.convert())
            }
            Self::_EVMError => near_primitives::errors::FunctionCallError::_EVMError,
            Self::ExecutionError(s) => {
                near_primitives::errors::FunctionCallError::ExecutionError(s)
            }
        }
    }
}

impl Converter<CompilationError> for near_primitives_crates_io::errors::CompilationError {
    fn convert(self) -> CompilationError {
        match self {
            Self::CodeDoesNotExist { account_id } => {
                near_primitives::errors::CompilationError::CodeDoesNotExist { account_id }
            }
            Self::PrepareError(prepare_error) => {
                near_primitives::errors::CompilationError::PrepareError(prepare_error.convert())
            }
            Self::WasmerCompileError { msg } => {
                near_primitives::errors::CompilationError::WasmerCompileError { msg }
            }
        }
    }
}

impl Converter<MethodResolveError> for near_primitives_crates_io::errors::MethodResolveError {
    fn convert(self) -> MethodResolveError {
        match self {
            Self::MethodEmptyName => near_primitives::errors::MethodResolveError::MethodEmptyName,
            Self::MethodNotFound => near_primitives::errors::MethodResolveError::MethodNotFound,
            Self::MethodInvalidSignature => {
                near_primitives::errors::MethodResolveError::MethodInvalidSignature
            }
        }
    }
}

impl Converter<PrepareError> for near_primitives_crates_io::errors::PrepareError {
    fn convert(self) -> PrepareError {
        match self {
            Self::Serialization => near_primitives::errors::PrepareError::Serialization,
            Self::Deserialization => near_primitives::errors::PrepareError::Deserialization,
            Self::InternalMemoryDeclared => {
                near_primitives::errors::PrepareError::InternalMemoryDeclared
            }
            Self::GasInstrumentation => near_primitives::errors::PrepareError::GasInstrumentation,
            Self::StackHeightInstrumentation => {
                near_primitives::errors::PrepareError::StackHeightInstrumentation
            }
            Self::Instantiate => near_primitives::errors::PrepareError::Instantiate,
            Self::Memory => near_primitives::errors::PrepareError::Memory,
            Self::TooManyFunctions => near_primitives::errors::PrepareError::TooManyFunctions,
            Self::TooManyLocals => near_primitives::errors::PrepareError::TooManyLocals,
        }
    }
}

impl Converter<WasmTrap> for near_primitives_crates_io::errors::WasmTrap {
    fn convert(self) -> WasmTrap {
        match self {
            Self::Unreachable => near_primitives::errors::WasmTrap::Unreachable,
            Self::IncorrectCallIndirectSignature => {
                near_primitives::errors::WasmTrap::IncorrectCallIndirectSignature
            }
            Self::MemoryOutOfBounds => near_primitives::errors::WasmTrap::MemoryOutOfBounds,
            Self::CallIndirectOOB => near_primitives::errors::WasmTrap::CallIndirectOOB,
            Self::IllegalArithmetic => near_primitives::errors::WasmTrap::IllegalArithmetic,
            Self::MisalignedAtomicAccess => {
                near_primitives::errors::WasmTrap::MisalignedAtomicAccess
            }
            Self::IndirectCallToNull => near_primitives::errors::WasmTrap::IndirectCallToNull,
            Self::StackOverflow => near_primitives::errors::WasmTrap::StackOverflow,
            Self::GenericTrap => near_primitives::errors::WasmTrap::GenericTrap,
        }
    }
}

impl Converter<HostError> for near_primitives_crates_io::errors::HostError {
    fn convert(self) -> HostError {
        match self {
            Self::BadUTF16 => near_primitives::errors::HostError::BadUTF16,
            Self::BadUTF8 => near_primitives::errors::HostError::BadUTF8,
            Self::GasExceeded => near_primitives::errors::HostError::GasExceeded,
            Self::GasLimitExceeded => near_primitives::errors::HostError::GasLimitExceeded,
            Self::BalanceExceeded => near_primitives::errors::HostError::BalanceExceeded,
            Self::EmptyMethodName => near_primitives::errors::HostError::EmptyMethodName,
            Self::GuestPanic { panic_msg } => {
                near_primitives::errors::HostError::GuestPanic { panic_msg }
            }
            Self::IntegerOverflow => near_primitives::errors::HostError::IntegerOverflow,
            Self::InvalidPromiseIndex { promise_idx } => {
                near_primitives::errors::HostError::InvalidPromiseIndex { promise_idx }
            }
            Self::CannotAppendActionToJointPromise => {
                near_primitives::errors::HostError::CannotAppendActionToJointPromise
            }
            Self::CannotReturnJointPromise => {
                near_primitives::errors::HostError::CannotReturnJointPromise
            }
            Self::InvalidPromiseResultIndex { result_idx } => {
                near_primitives::errors::HostError::InvalidPromiseResultIndex { result_idx }
            }
            Self::InvalidRegisterId { register_id } => {
                near_primitives::errors::HostError::InvalidRegisterId { register_id }
            }
            Self::IteratorWasInvalidated { iterator_index } => {
                near_primitives::errors::HostError::IteratorWasInvalidated { iterator_index }
            }
            Self::MemoryAccessViolation => {
                near_primitives::errors::HostError::MemoryAccessViolation
            }
            Self::InvalidReceiptIndex { receipt_index } => {
                near_primitives::errors::HostError::InvalidReceiptIndex { receipt_index }
            }
            Self::InvalidIteratorIndex { iterator_index } => {
                near_primitives::errors::HostError::InvalidIteratorIndex { iterator_index }
            }
            Self::InvalidAccountId => near_primitives::errors::HostError::InvalidAccountId,
            Self::InvalidMethodName => near_primitives::errors::HostError::InvalidMethodName,
            Self::InvalidPublicKey => near_primitives::errors::HostError::InvalidPublicKey,
            Self::ProhibitedInView { method_name } => {
                near_primitives::errors::HostError::ProhibitedInView { method_name }
            }
            Self::NumberOfLogsExceeded { limit } => {
                near_primitives::errors::HostError::NumberOfLogsExceeded { limit }
            }
            Self::KeyLengthExceeded { length, limit } => {
                near_primitives::errors::HostError::KeyLengthExceeded { length, limit }
            }
            Self::ValueLengthExceeded { length, limit } => {
                near_primitives::errors::HostError::ValueLengthExceeded { length, limit }
            }
            Self::TotalLogLengthExceeded { length, limit } => {
                near_primitives::errors::HostError::TotalLogLengthExceeded { length, limit }
            }
            Self::NumberPromisesExceeded {
                number_of_promises,
                limit,
            } => near_primitives::errors::HostError::NumberPromisesExceeded {
                number_of_promises,
                limit,
            },
            Self::NumberInputDataDependenciesExceeded {
                number_of_input_data_dependencies,
                limit,
            } => near_primitives::errors::HostError::NumberInputDataDependenciesExceeded {
                number_of_input_data_dependencies,
                limit,
            },
            Self::ReturnedValueLengthExceeded { length, limit } => {
                near_primitives::errors::HostError::ReturnedValueLengthExceeded { length, limit }
            }
            Self::ContractSizeExceeded { size, limit } => {
                near_primitives::errors::HostError::ContractSizeExceeded { size, limit }
            }
            Self::Deprecated { method_name } => {
                near_primitives::errors::HostError::Deprecated { method_name }
            }
            Self::ECRecoverError { msg } => {
                near_primitives::errors::HostError::ECRecoverError { msg }
            }
            Self::AltBn128InvalidInput { msg } => {
                near_primitives::errors::HostError::AltBn128InvalidInput { msg }
            }
            Self::Ed25519VerifyInvalidInput { msg } => {
                near_primitives::errors::HostError::Ed25519VerifyInvalidInput { msg }
            }
        }
    }
}

impl Converter<ReceiptValidationError>
    for near_primitives_crates_io::errors::ReceiptValidationError
{
    fn convert(self) -> ReceiptValidationError {
        match self {
                Self::InvalidPredecessorId { account_id } =>{
                    near_primitives::errors::ReceiptValidationError::InvalidPredecessorId { account_id }
                }
                Self::InvalidReceiverId { account_id } =>{
                    near_primitives::errors::ReceiptValidationError::InvalidReceiverId { account_id }
                }
                Self::InvalidSignerId { account_id } =>{
                    near_primitives::errors::ReceiptValidationError::InvalidSignerId { account_id }
                }
                Self::InvalidDataReceiverId { account_id } =>{
                    near_primitives::errors::ReceiptValidationError::InvalidDataReceiverId { account_id }
                }
                Self::ReturnedValueLengthExceeded { length, limit } =>{
                    near_primitives::errors::ReceiptValidationError::ReturnedValueLengthExceeded { length, limit }
                }
                Self::NumberInputDataDependenciesExceeded { number_of_input_data_dependencies, limit } =>{
                    near_primitives::errors::ReceiptValidationError::NumberInputDataDependenciesExceeded { number_of_input_data_dependencies, limit }
                }
                Self::ActionsValidation(e) =>{
                    near_primitives::errors::ReceiptValidationError::ActionsValidation(e.convert())
                }
                Self::ReceiptSizeExceeded { size, limit } =>{
                    near_primitives::errors::ReceiptValidationError::ReceiptSizeExceeded { size, limit }
                }
            }
    }
}

impl Converter<ActionsValidationError>
    for near_primitives_crates_io::errors::ActionsValidationError
{
    fn convert(self) -> ActionsValidationError {
        match self {
                Self::DeleteActionMustBeFinal =>
                    near_primitives::errors::ActionsValidationError::DeleteActionMustBeFinal,
                Self::TotalPrepaidGasExceeded { total_prepaid_gas, limit } =>
                    near_primitives::errors::ActionsValidationError::TotalPrepaidGasExceeded { total_prepaid_gas, limit },
                Self::TotalNumberOfActionsExceeded { total_number_of_actions, limit } =>
                    near_primitives::errors::ActionsValidationError::TotalNumberOfActionsExceeded { total_number_of_actions, limit },
                Self::AddKeyMethodNamesNumberOfBytesExceeded { total_number_of_bytes, limit } =>
                    near_primitives::errors::ActionsValidationError::AddKeyMethodNamesNumberOfBytesExceeded { total_number_of_bytes, limit },
                Self::AddKeyMethodNameLengthExceeded { length, limit } =>
                    near_primitives::errors::ActionsValidationError::AddKeyMethodNameLengthExceeded { length, limit },
                Self::IntegerOverflow =>
                    near_primitives::errors::ActionsValidationError::IntegerOverflow,
                Self::InvalidAccountId { account_id } =>
                    near_primitives::errors::ActionsValidationError::InvalidAccountId { account_id },
                Self::ContractSizeExceeded { size, limit } =>
                    near_primitives::errors::ActionsValidationError::ContractSizeExceeded { size, limit },
                Self::FunctionCallMethodNameLengthExceeded { length, limit } =>
                    near_primitives::errors::ActionsValidationError::FunctionCallMethodNameLengthExceeded { length, limit },
                Self::FunctionCallArgumentsLengthExceeded { length, limit } =>
                    near_primitives::errors::ActionsValidationError::FunctionCallArgumentsLengthExceeded { length, limit },
                Self::UnsuitableStakingKey { public_key } =>
                    near_primitives::errors::ActionsValidationError::UnsuitableStakingKey { public_key: Box::new(public_key.convert()) },
                Self::FunctionCallZeroAttachedGas =>
                    near_primitives::errors::ActionsValidationError::FunctionCallZeroAttachedGas,
                Self::DelegateActionMustBeOnlyOne =>
                    near_primitives::errors::ActionsValidationError::DelegateActionMustBeOnlyOne,
                Self::UnsupportedProtocolFeature { protocol_feature, version } =>
                    near_primitives::errors::ActionsValidationError::UnsupportedProtocolFeature { protocol_feature, version },
            }
    }
}

impl Converter<StorageError> for near_primitives_crates_io::errors::StorageError {
    fn convert(self) -> StorageError {
        match self {
            Self::StorageInternalError => {
                near_primitives::errors::StorageError::StorageInternalError
            }
            Self::MissingTrieValue(missing_trie_value_context, crypto_hash) => {
                near_primitives::errors::StorageError::MissingTrieValue(
                    missing_trie_value_context.convert(),
                    crypto_hash.convert(),
                )
            }
            Self::UnexpectedTrieValue => near_primitives::errors::StorageError::UnexpectedTrieValue,
            Self::StorageInconsistentState(s) => {
                near_primitives::errors::StorageError::StorageInconsistentState(s)
            }
            Self::FlatStorageBlockNotSupported(s) => {
                near_primitives::errors::StorageError::FlatStorageBlockNotSupported(s)
            }
            Self::MemTrieLoadingError(s) => {
                near_primitives::errors::StorageError::MemTrieLoadingError(s)
            }
            Self::FlatStorageReshardingAlreadyInProgress => {
                near_primitives::errors::StorageError::FlatStorageReshardingAlreadyInProgress
            }
        }
    }
}

impl Converter<MissingTrieValueContext>
    for near_primitives_crates_io::errors::MissingTrieValueContext
{
    fn convert(self) -> MissingTrieValueContext {
        match self {
            Self::TrieIterator => near_primitives::errors::MissingTrieValueContext::TrieIterator,
            Self::TriePrefetchingStorage => {
                near_primitives::errors::MissingTrieValueContext::TriePrefetchingStorage
            }
            Self::TrieMemoryPartialStorage => {
                near_primitives::errors::MissingTrieValueContext::TrieMemoryPartialStorage
            }
            Self::TrieStorage => near_primitives::errors::MissingTrieValueContext::TrieStorage,
        }
    }
}

impl Converter<InvalidAccessKeyError> for near_primitives_crates_io::errors::InvalidAccessKeyError {
    fn convert(self) -> InvalidAccessKeyError {
        match self {
            Self::AccessKeyNotFound {
                account_id,
                public_key,
            } => near_primitives::errors::InvalidAccessKeyError::AccessKeyNotFound {
                account_id,
                public_key: Box::new(public_key.convert()),
            },
            Self::ReceiverMismatch {
                tx_receiver,
                ak_receiver,
            } => near_primitives::errors::InvalidAccessKeyError::ReceiverMismatch {
                tx_receiver,
                ak_receiver,
            },
            Self::MethodNameMismatch { method_name } => {
                near_primitives::errors::InvalidAccessKeyError::MethodNameMismatch { method_name }
            }
            Self::RequiresFullAccess => {
                near_primitives::errors::InvalidAccessKeyError::RequiresFullAccess
            }
            Self::NotEnoughAllowance {
                account_id,
                public_key,
                allowance,
                cost,
            } => near_primitives::errors::InvalidAccessKeyError::NotEnoughAllowance {
                account_id,
                public_key: Box::new(public_key.convert()),
                allowance,
                cost,
            },
            Self::DepositWithFunctionCall => {
                near_primitives::errors::InvalidAccessKeyError::DepositWithFunctionCall
            }
        }
    }
}

//
// From IndexerShard to Shard
//

impl Converter<StateChangesView>
    for near_lake_framework::near_indexer_primitives::views::StateChangesView
{
    fn convert(self) -> StateChangesView {
        self.into_iter().map(Converter::convert).collect()
    }
}

impl Converter<StateChangeWithCauseView>
    for near_lake_framework::near_indexer_primitives::views::StateChangeWithCauseView
{
    fn convert(self) -> StateChangeWithCauseView {
        StateChangeWithCauseView {
            cause: self.cause.convert(),
            value: self.value.convert(),
        }
    }
}

impl Converter<StateChangeCauseView>
    for near_lake_framework::near_indexer_primitives::views::StateChangeCauseView
{
    fn convert(self) -> StateChangeCauseView {
        match self {
            Self::NotWritableToDisk => StateChangeCauseView::NotWritableToDisk,
            Self::InitialState => StateChangeCauseView::InitialState,
            Self::TransactionProcessing { tx_hash } => {
                StateChangeCauseView::TransactionProcessing {
                    tx_hash: tx_hash.convert(),
                }
            }
            Self::ActionReceiptProcessingStarted { receipt_hash } => {
                StateChangeCauseView::ActionReceiptProcessingStarted {
                    receipt_hash: receipt_hash.convert(),
                }
            }
            Self::ActionReceiptGasReward { receipt_hash } => {
                StateChangeCauseView::ActionReceiptGasReward {
                    receipt_hash: receipt_hash.convert(),
                }
            }
            Self::ReceiptProcessing { receipt_hash } => StateChangeCauseView::ReceiptProcessing {
                receipt_hash: receipt_hash.convert(),
            },
            Self::PostponedReceipt { receipt_hash } => StateChangeCauseView::PostponedReceipt {
                receipt_hash: receipt_hash.convert(),
            },
            Self::UpdatedDelayedReceipts => StateChangeCauseView::UpdatedDelayedReceipts,
            Self::ValidatorAccountsUpdate => StateChangeCauseView::ValidatorAccountsUpdate,
            Self::Migration => StateChangeCauseView::Migration,
            Self::ReshardingV2 => StateChangeCauseView::ReshardingV2,
            Self::BandwidthSchedulerStateUpdate => {
                StateChangeCauseView::BandwidthSchedulerStateUpdate
            }
        }
    }
}

impl Converter<StateChangeValueView>
    for near_lake_framework::near_indexer_primitives::views::StateChangeValueView
{
    fn convert(self) -> StateChangeValueView {
        match self {
            Self::AccountUpdate {
                account_id,
                account,
            } => StateChangeValueView::AccountUpdate {
                account_id,
                account: account.convert(),
            },
            Self::AccountDeletion { account_id } => {
                StateChangeValueView::AccountDeletion { account_id }
            }
            Self::AccessKeyUpdate {
                account_id,
                public_key,
                access_key,
            } => StateChangeValueView::AccessKeyUpdate {
                account_id,
                public_key: public_key.convert(),
                access_key: access_key.convert(),
            },
            Self::AccessKeyDeletion {
                account_id,
                public_key,
            } => StateChangeValueView::AccessKeyDeletion {
                account_id,
                public_key: public_key.convert(),
            },
            Self::DataUpdate {
                account_id,
                key,
                value,
            } => StateChangeValueView::DataUpdate {
                account_id,
                key: key.convert(),
                value: value.convert(),
            },
            Self::DataDeletion { account_id, key } => StateChangeValueView::DataDeletion {
                account_id,
                key: key.convert(),
            },
            Self::ContractCodeUpdate { account_id, code } => {
                StateChangeValueView::ContractCodeUpdate { account_id, code }
            }
            Self::ContractCodeDeletion { account_id } => {
                StateChangeValueView::ContractCodeDeletion { account_id }
            }
        }
    }
}

impl Converter<AccountView> for near_primitives_crates_io::views::AccountView {
    fn convert(self) -> AccountView {
        AccountView {
            amount: self.amount,
            locked: self.locked,
            code_hash: self.code_hash.convert(),
            storage_usage: self.storage_usage,
            storage_paid_at: self.storage_paid_at,
            global_contract_hash: self.global_contract_hash.map(Converter::convert),
            global_contract_account_id: self.global_contract_account_id,
        }
    }
}
