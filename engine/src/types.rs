use engine_standalone_storage::sync::types::TransactionKind;
use strum::EnumString;

#[derive(EnumString)]
pub enum InnerTransactionKind {
    #[strum(serialize = "submit")]
    Submit,
    #[strum(serialize = "call")]
    Call,
    #[strum(serialize = "pause_precompiles")]
    PausePrecompiles,
    #[strum(serialize = "resume_precompiles")]
    ResumePrecompiles,
    #[strum(serialize = "deploy_code")]
    Deploy,
    #[strum(serialize = "deploy_erc20_token")]
    DeployErc20,
    #[strum(serialize = "ft_on_transfer")]
    FtOnTransfer,
    #[strum(serialize = "deposit")]
    Deposit,
    #[strum(serialize = "ft_transfer_call")]
    FtTransferCall,
    #[strum(serialize = "finish_deposit")]
    FinishDeposit,
    #[strum(serialize = "ft_resolve_transfer")]
    ResolveTransfer,
    #[strum(serialize = "ft_transfer")]
    FtTransfer,
    #[strum(serialize = "withdraw")]
    Withdraw,
    #[strum(serialize = "storage_deposit")]
    StorageDeposit,
    #[strum(serialize = "storage_unregister")]
    StorageUnregister,
    #[strum(serialize = "storage_withdraw")]
    StorageWithdraw,
    #[strum(serialize = "set_paused_flags")]
    SetPausedFlags,
    #[strum(serialize = "register_relayer")]
    RegisterRelayer,
    #[strum(serialize = "refund_on_error")]
    RefundOnError,
    #[strum(serialize = "set_eth_connector_contract_data")]
    SetConnectorData,
    #[strum(serialize = "new_eth_connector")]
    NewConnector,
    #[strum(serialize = "new")]
    NewEngine,
    #[strum(serialize = "factory_update")]
    FactoryUpdate,
    #[strum(serialize = "factory_update_address_version")]
    FactoryUpdateAddressVersion,
    #[strum(serialize = "factory_set_wnear_address")]
    FactorySetWNearAddress,
    #[strum(serialize = "set_owner")]
    SetOwner,
    #[strum(serialize = "submit_with_args")]
    SubmitWithArgs,
    #[strum(serialize = "set_upgrade_delay_blocks")]
    SetUpgradeDelayBlocks,
    #[strum(serialize = "fund_xcc_sub_account")]
    FundXccSubAccound,
    #[strum(serialize = "pause_contract")]
    PauseContract,
    #[strum(serialize = "resume_contract")]
    ResumeContract,
    #[strum(serialize = "set_key_manager")]
    SetKeyManager,
    #[strum(serialize = "add_relayer_key")]
    AddRelayerKey,
    #[strum(serialize = "remove_relayer_key")]
    RemoveRelayerKey,
    #[strum(serialize = "set_eth_connector_contract_account")]
    SetEthConnectorContractAccount,
    #[strum(serialize = "disable_legacy_nep141")]
    DisableLegacyNEP141,
    #[strum(serialize = "set_fixed_gas_cost")]
    SetFixedGasCost,
    #[strum(serialize = "set_silo_params")]
    SetSiloParams,
    #[strum(serialize = "set_whitelist_status")]
    SetWhitelistStatus,
    #[strum(serialize = "add_entry_to_whitelist")]
    AddEntryToWhitelist,
    #[strum(serialize = "add_entry_to_whitelist_batch")]
    AddEntryToWhitelistBatch,
    #[strum(serialize = "remove_entry_from_whitelist")]
    RemoveEntryFromWhitelist,
    Unknown,
}

/// Used to make sure InnerTransactionKind is kept in sync with TransactionKind
impl From<TransactionKind> for InnerTransactionKind {
    fn from(tx: TransactionKind) -> Self {
        match tx {
            TransactionKind::Submit(_) => InnerTransactionKind::Submit,
            TransactionKind::Call(_) => InnerTransactionKind::Call,
            TransactionKind::PausePrecompiles(_) => InnerTransactionKind::PausePrecompiles,
            TransactionKind::ResumePrecompiles(_) => InnerTransactionKind::ResumePrecompiles,
            TransactionKind::Deploy(_) => InnerTransactionKind::Deploy,
            TransactionKind::DeployErc20(_) => InnerTransactionKind::DeployErc20,
            TransactionKind::FtOnTransfer(_) => InnerTransactionKind::FtOnTransfer,
            TransactionKind::Deposit(_) => InnerTransactionKind::Deposit,
            TransactionKind::FtTransferCall(_) => InnerTransactionKind::FtTransferCall,
            TransactionKind::FinishDeposit(_) => InnerTransactionKind::FinishDeposit,
            TransactionKind::ResolveTransfer(_, _) => InnerTransactionKind::ResolveTransfer,
            TransactionKind::FtTransfer(_) => InnerTransactionKind::FtTransfer,
            TransactionKind::Withdraw(_) => InnerTransactionKind::Withdraw,
            TransactionKind::StorageDeposit(_) => InnerTransactionKind::StorageDeposit,
            TransactionKind::StorageUnregister(_) => InnerTransactionKind::StorageUnregister,
            TransactionKind::StorageWithdraw(_) => InnerTransactionKind::StorageWithdraw,
            TransactionKind::SetPausedFlags(_) => InnerTransactionKind::SetPausedFlags,
            TransactionKind::RegisterRelayer(_) => InnerTransactionKind::RegisterRelayer,
            TransactionKind::RefundOnError(_) => InnerTransactionKind::RefundOnError,
            TransactionKind::SetConnectorData(_) => InnerTransactionKind::SetConnectorData,
            TransactionKind::NewConnector(_) => InnerTransactionKind::NewConnector,
            TransactionKind::NewEngine(_) => InnerTransactionKind::NewEngine,
            TransactionKind::FactoryUpdate(_) => InnerTransactionKind::FactoryUpdate,
            TransactionKind::FactoryUpdateAddressVersion(_) => {
                InnerTransactionKind::FactoryUpdateAddressVersion
            }
            TransactionKind::FactorySetWNearAddress(_) => {
                InnerTransactionKind::FactorySetWNearAddress
            }
            TransactionKind::SetOwner(_) => InnerTransactionKind::SetOwner,
            TransactionKind::SubmitWithArgs(_) => InnerTransactionKind::SubmitWithArgs,
            TransactionKind::SetUpgradeDelayBlocks(_) => {
                InnerTransactionKind::SetUpgradeDelayBlocks
            }
            TransactionKind::FundXccSubAccount(_) => InnerTransactionKind::FundXccSubAccound,
            TransactionKind::PauseContract => InnerTransactionKind::PauseContract,
            TransactionKind::ResumeContract => InnerTransactionKind::ResumeContract,
            TransactionKind::SetKeyManager(_) => InnerTransactionKind::SetKeyManager,
            TransactionKind::AddRelayerKey(_) => InnerTransactionKind::AddRelayerKey,
            TransactionKind::RemoveRelayerKey(_) => InnerTransactionKind::RemoveRelayerKey,
            TransactionKind::SetEthConnectorContractAccount(_) => {
                InnerTransactionKind::SetEthConnectorContractAccount
            }
            TransactionKind::DisableLegacyNEP141 => InnerTransactionKind::DisableLegacyNEP141,
            TransactionKind::SetFixedGasCost(_) => InnerTransactionKind::SetFixedGasCost,
            TransactionKind::SetSiloParams(_) => InnerTransactionKind::SetSiloParams,
            TransactionKind::AddEntryToWhitelist(_) => InnerTransactionKind::AddEntryToWhitelist,
            TransactionKind::AddEntryToWhitelistBatch(_) => {
                InnerTransactionKind::AddEntryToWhitelistBatch
            }
            TransactionKind::RemoveEntryFromWhitelist(_) => {
                InnerTransactionKind::RemoveEntryFromWhitelist
            }
            TransactionKind::SetWhitelistStatus(_) => InnerTransactionKind::SetWhitelistStatus,
            TransactionKind::Unknown => InnerTransactionKind::Unknown,
        }
    }
}
