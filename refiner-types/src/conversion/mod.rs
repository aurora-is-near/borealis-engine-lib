use near_crypto::{ED25519PublicKey, PublicKey, Secp256K1PublicKey, Signature};
use near_primitives::action::delegate::{DelegateAction, NonDelegateAction};
use near_primitives::action::{
    Action, CreateAccountAction, DeployContractAction, FunctionCallAction, TransferAction,
};
use near_primitives::hash::CryptoHash;
use near_primitives::types::{AccountId, ShardId};
use near_primitives::views::{self, ActionView, DataReceiverView, ReceiptEnumView, ReceiptView};
use std::str::FromStr;

pub trait Converter<T> {
    fn convert(&self) -> T;
}

impl Converter<ShardId> for near_primitives_crates::types::ShardId {
    fn convert(&self) -> ShardId {
        ShardId::new((*self).into())
    }
}

impl Converter<AccountId> for near_primitives_crates::types::AccountId {
    fn convert(&self) -> AccountId {
        AccountId::from_str(self.as_str()).unwrap()
    }
}

impl Converter<CryptoHash> for near_primitives_crates::hash::CryptoHash {
    fn convert(&self) -> CryptoHash {
        CryptoHash(self.0)
    }
}

impl Converter<Signature> for near_crypto_crates::Signature {
    fn convert(&self) -> Signature {
        match self {
            near_crypto_crates::Signature::ED25519(s) => Signature::ED25519(s.clone()),
            near_crypto_crates::Signature::SECP256K1(s) => {
                let s: [u8; 65] = s.clone().into();
                Signature::SECP256K1(near_crypto::Secp256K1Signature::from(s))
            }
        }
    }
}

impl Converter<PublicKey> for near_crypto_crates::PublicKey {
    fn convert(&self) -> PublicKey {
        match self {
            Self::ED25519(s) => PublicKey::ED25519(ED25519PublicKey(s.0)),
            Self::SECP256K1(s) => {
                PublicKey::SECP256K1(Secp256K1PublicKey::try_from(s.as_ref()).unwrap())
            }
        }
    }
}

impl Converter<views::validator_stake_view::ValidatorStakeView>
    for near_primitives_crates::views::validator_stake_view::ValidatorStakeView
{
    fn convert(&self) -> views::validator_stake_view::ValidatorStakeView {
        let Self::V1(validator_stake_view) = self;
        views::validator_stake_view::ValidatorStakeView::V1(
            views::validator_stake_view::ValidatorStakeViewV1 {
                account_id: validator_stake_view.account_id.convert(),
                public_key: validator_stake_view.public_key.convert(),
                stake: validator_stake_view.stake,
            },
        )
    }
}

impl Converter<ReceiptView> for near_primitives_crates::views::ReceiptView {
    fn convert(&self) -> ReceiptView {
        ReceiptView {
            receipt_id: self.receipt_id.convert(),
            receipt: self.receipt.convert(),
            predecessor_id: self.predecessor_id.convert(),
            receiver_id: self.receiver_id.convert(),
            priority: self.priority,
        }
    }
}

impl Converter<ReceiptEnumView> for near_primitives_crates::views::ReceiptEnumView {
    fn convert(&self) -> ReceiptEnumView {
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
                signer_id: signer_id.convert(),
                signer_public_key: signer_public_key.convert(),
                gas_price: *gas_price,
                output_data_receivers: output_data_receivers
                    .iter()
                    .map(Converter::convert)
                    .collect(),
                input_data_ids: input_data_ids.iter().map(Converter::convert).collect(),
                actions: actions.iter().map(Converter::convert).collect(),
                is_promise_yield: *is_promise_yield,
            },
            Self::Data {
                data_id,
                data,
                is_promise_resume,
            } => ReceiptEnumView::Data {
                data_id: data_id.convert(),
                data: data.clone(),
                is_promise_resume: *is_promise_resume,
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
                    .iter()
                    .map(Converter::convert)
                    .collect(),
                code: code.clone(),
            },
        }
    }
}

impl Converter<DataReceiverView> for near_primitives_crates::views::DataReceiverView {
    fn convert(&self) -> DataReceiverView {
        DataReceiverView {
            data_id: self.data_id.convert(),
            receiver_id: self.receiver_id.convert(),
        }
    }
}

impl Converter<ActionView> for near_primitives_crates::views::ActionView {
    fn convert(&self) -> ActionView {
        match self {
            near_primitives_crates::views::ActionView::CreateAccount => ActionView::CreateAccount,
            near_primitives_crates::views::ActionView::DeployContract { code } => {
                ActionView::DeployContract { code: code.clone() }
            }
            near_primitives_crates::views::ActionView::FunctionCall {
                method_name,
                args,
                gas,
                deposit,
            } => ActionView::FunctionCall {
                method_name: method_name.clone(),
                args: near_primitives::types::FunctionArgs::from(args.clone().into()),
                gas: *gas,
                deposit: *deposit,
            },
            near_primitives_crates::views::ActionView::Transfer { deposit } => {
                ActionView::Transfer { deposit: *deposit }
            }
            near_primitives_crates::views::ActionView::Stake { stake, public_key } => {
                ActionView::Stake {
                    stake: *stake,
                    public_key: public_key.convert(),
                }
            }
            near_primitives_crates::views::ActionView::AddKey { .. } => {}
            near_primitives_crates::views::ActionView::DeleteKey { public_key } => {
                ActionView::DeleteKey {
                    public_key: public_key.convert(),
                }
            }
            near_primitives_crates::views::ActionView::DeleteAccount { beneficiary_id } => {
                ActionView::DeleteAccount {
                    beneficiary_id: beneficiary_id.convert(),
                }
            }
            near_primitives_crates::views::ActionView::Delegate {
                delegate_action,
                signature,
            } => ActionView::Delegate {
                delegate_action: delegate_action.convert(),
                signature: signature.convert(),
            },
            near_primitives_crates::views::ActionView::DeployGlobalContract { code } => {
                ActionView::DeployGlobalContract { code: code.clone() }
            }
            near_primitives_crates::views::ActionView::DeployGlobalContractByAccountId { code } => {
                ActionView::DeployGlobalContractByAccountId { code: code.clone() }
            }
            near_primitives_crates::views::ActionView::UseGlobalContract { code_hash } => {
                ActionView::UseGlobalContract {
                    code_hash: code_hash.convert(),
                }
            }
            near_primitives_crates::views::ActionView::UseGlobalContractByAccountId {
                account_id,
            } => ActionView::UseGlobalContractByAccountId {
                account_id: account_id.convert(),
            },
        }
    }
}

impl Converter<DelegateAction> for near_primitives_crates::action::delegate::DelegateAction {
    fn convert(&self) -> DelegateAction {
        DelegateAction {
            sender_id: self.sender_id.convert(),
            receiver_id: self.receiver_id.convert(),
            actions: self.actions.iter().map(Converter::convert).collect(),
            nonce: self.nonce,
            max_block_height: self.max_block_height,
            public_key: self.public_key.convert(),
        }
    }
}

impl Converter<NonDelegateAction> for near_primitives_crates::action::delegate::NonDelegateAction {
    fn convert(&self) -> NonDelegateAction {
        let action: near_primitives_crates::action::Action = self.clone().try_into().unwrap();
        NonDelegateAction::try_from(action.convert()).unwrap()
    }
}

impl Converter<Action> for near_primitives_crates::action::Action {
    fn convert(&self) -> Action {
        match self {
            Self::CreateAccount(_) => Action::CreateAccount(CreateAccountAction {}),
            Self::DeployContract(a) => Action::DeployContract(DeployContractAction {
                code: a.code.clone(),
            }),
            Self::FunctionCall(a) => Action::FunctionCall(Box::new(FunctionCallAction {
                method_name: a.method_name.clone(),
                args: a.args.clone(),
                gas: a.gas,
                deposit: a.deposit,
            })),
            Self::Transfer(a) => Action::Transfer(TransferAction { deposit: a.deposit }),
            Self::Stake(_) => {}
            Self::AddKey(_) => {}
            Self::DeleteKey(_) => {}
            Self::DeleteAccount(_) => {}
            Self::Delegate(_) => {}
            Self::DeployGlobalContract(_) => {}
            Self::UseGlobalContract(_) => {}
        }
    }
}
