use engine_standalone_storage::sync::types::TransactionKindTag;
use lazy_static::lazy_static;
use prometheus::{
    self, labels, opts, register_int_counter, register_int_gauge, IntCounter, IntGauge, Opts,
};

lazy_static! {
    pub static ref MISSING_SHARDS: IntCounter = counter(
        "refiner_missing_shards",
        "Blocks that are missing shards"
    );
    pub static ref BATCHED_ACTIONS: IntCounter = counter(
        "refiner_batched_actions",
        "Transactions that uses batched actions"
    );
    pub static ref ERROR_BUILDING_TRANSACTION: IntCounter = counter(
        "refiner_error_building_transaction",
        "Error building transaction"
    );
    pub static ref LATEST_BLOCK_PROCESSED: IntGauge = gauge(
        "refiner_number_of_latest_block_processed",
        "Height of last block processed. Can be slightly out of sync with the actual height given multiple process"
    );
    pub static ref FAILING_NEAR_TRANSACTION: IntCounter = counter(
        "refiner_near_transaction_failed",
        "NEAR Transaction failed"
    );
    pub static ref TRANSACTIONS: IntCounter = counter(
        "refiner_transactions",
        "Number of transactions after filter"
    );
    pub static ref TRANSACTIONS_ACTION: IntCounter = counter(
        "refiner_transaction_actions",
        "Number of actions inside transactions"
    );
    pub static ref TRANSACTIONS_DATA: IntCounter = counter(
        "refiner_transaction_data",
        "Number of receipts that are of type data"
    );
    pub static ref TRANSACTION_TYPE_SUBMIT: IntCounter = counter(
        "refiner_tx_type_submit",
        "Number of transactions of type: submit"
    );
    pub static ref TRANSACTION_TYPE_SUBMIT_WITH_ARGS: IntCounter = counter(
        "refiner_tx_type_submit_with_args",
        "Number of transactions of type: submit_with_args"
    );
    pub static ref TRANSACTION_TYPE_CALL: IntCounter = counter(
        "refiner_tx_type_call",
        "Number of transactions of type: call"
    );
    pub static ref TRANSACTION_TYPE_PAUSE_PRECOMPILES: IntCounter = counter(
        "refiner_tx_type_pause_precompiles",
        "Number of transactions of type: pause_precompiles"
    );
    pub static ref TRANSACTION_TYPE_RESUME_PRECOMPILES: IntCounter = counter(
        "refiner_tx_type_resume_precompiles",
        "Number of transactions of type: resume_precompiles"
    );
    pub static ref TRANSACTION_TYPE_SET_OWNER: IntCounter = counter(
        "refiner_tx_type_set_owner",
        "Number of transactions of type: set_owner"
    );
    pub static ref TRANSACTION_TYPE_DEPLOY_CODE: IntCounter = counter(
        "refiner_tx_type_deploy_code",
        "Number of transactions of type: deploy_code"
    );
    pub static ref TRANSACTION_TYPE_DEPLOY_ERC20_TOKEN: IntCounter = counter(
        "refiner_tx_type_deploy_erc20_token",
        "Number of transactions of type: deploy_erc20_token"
    );
    pub static ref TRANSACTION_TYPE_FT_ON_TRANSFER: IntCounter = counter(
        "refiner_tx_type_ft_on_transfer",
        "Number of transactions of type: ft_on_transfer"
    );
    pub static ref TRANSACTION_TYPE_DEPOSIT: IntCounter = counter(
        "refiner_tx_type_deposit",
        "Number of transactions of type: deposit"
    );
    pub static ref TRANSACTION_TYPE_WITHDRAW: IntCounter = counter(
        "refiner_tx_type_withdraw",
        "Number of transactions of type: withdraw"
    );
    pub static ref TRANSACTION_TYPE_FINISH_DEPOSIT: IntCounter = counter(
        "refiner_tx_type_finish_deposit",
        "Number of transactions of type: finish_deposit"
    );
    pub static ref TRANSACTION_TYPE_FT_TRANSFER: IntCounter = counter(
        "refiner_tx_type_ft_transfer",
        "Number of transactions of type: ft_deposit"
    );
    pub static ref TRANSACTION_TYPE_FT_TRANSFER_CALL: IntCounter = counter(
        "refiner_tx_type_ft_transfer_call",
        "Number of transactions of type: ft_transfer_call"
    );
    pub static ref TRANSACTION_TYPE_FT_RESOLVE_TRANSFER: IntCounter = counter(
        "refiner_tx_type_ft_resolve_transfer",
        "Number of transactions of type: ft_resolve_transfer"
    );
    pub static ref TRANSACTION_TYPE_STORAGE_DEPOSIT: IntCounter = counter(
        "refiner_tx_type_storage_deposit",
        "Number of transactions of type: storage_deposit"
    );
    pub static ref TRANSACTION_TYPE_STORAGE_UNREGISTER: IntCounter = counter(
        "refiner_tx_type_storage_unregister",
        "Number of transactions of type: storage_unregister"
    );
    pub static ref TRANSACTION_TYPE_STORAGE_WITHDRAW: IntCounter = counter(
        "refiner_tx_type_storage_deposit",
        "Number of transactions of type: storage_deposit"
    );
    pub static ref TRANSACTION_TYPE_SET_PAUSED_FLAGS: IntCounter = counter(
        "refiner_tx_type_set_paused_flags",
        "Number of transactions of type: set_paused_flags"
    );
    pub static ref TRANSACTION_TYPE_REGISTER_RELAYER: IntCounter = counter(
        "refiner_tx_type_register_relayer",
        "Number of transactions of type: register_relayer"
    );
    pub static ref TRANSACTION_TYPE_REFUND_ON_ERROR: IntCounter = counter(
        "refiner_tx_type_refund_on_error",
        "Number of transactions of type: refund_on_error"
    );
    pub static ref TRANSACTION_TYPE_SET_CONNECTOR_DATA: IntCounter = counter(
        "refiner_tx_type_set_connector_data",
        "Number of transactions of type: set_connector_data"
    );
    pub static ref TRANSACTION_TYPE_NEW_CONNECTOR: IntCounter = counter(
        "refiner_tx_type_new_connector",
        "Number of transactions of type: new_connector"
    );
    pub static ref TRANSACTION_TYPE_NEW_ENGINE: IntCounter = counter(
        "refiner_tx_type_new_engine",
        "Number of transactions of type: new_engine"
    );
    pub static ref TRANSACTION_TYPE_SET_UPGRADE_DELAY_BLOCKS: IntCounter = counter(
        "refiner_tx_type_set_upgrade_delay_blocks",
        "Number of transactions of type: set_upgrade_delay_blocks"
    );
    pub static ref TRANSACTION_TYPE_FUND_XCC_SUB_ACCOUNT: IntCounter = counter(
        "refiner_tx_type_fund_xcc_sub_account",
        "Number of transactions of type: fund_xcc_sub_account"
    );
    pub static ref TRANSACTION_TYPE_UNKNOWN: IntCounter = counter(
        "refiner_tx_type_unknown",
        "Number of transactions of type: unknown"
    );
    pub static ref SKIP_BLOCKS: IntCounter = counter(
        "refiner_near_listener_skip_blocks",
        "Number of skip blocks seen"
    );
    pub static ref PROCESSED_BLOCKS: IntCounter = counter(
        "refiner_near_listener_processed_blocks",
        "Number of blocks processed"
    );
    pub static ref UNKNOWN_TX_FOR_RECEIPT: IntCounter = counter(
        "refiner_unknown_tx_for_receipt",
        "Number of receipts where the transaction provenance was not known (should be 0)"
    );
    pub static ref TRANSACTION_TYPE_FACTORY_UPDATE: IntCounter = counter(
        "refiner_tx_type_factory_update",
        "Number of transactions of type: factory_update"
    );
    pub static ref TRANSACTION_TYPE_FACTORY_UPDATE_ADDRESS_VERSION: IntCounter = counter(
        "refiner_tx_type_factory_update_address_version",
        "Number of transactions of type: factory_update_address_version"
    );
    pub static ref TRANSACTION_TYPE_FACTORY_SET_WNEAR_ADDRESS: IntCounter = counter(
        "refiner_tx_type_factory_set_wnear_address",
        "Number of transactions of type: factory_set_wnear_address"
    );
    pub static ref TRANSACTION_TYPE_PAUSE_CONTRACT: IntCounter = counter(
        "refiner_tx_type_pause_contract",
        "Number of transactions of type: pause_contract"
    );
    pub static ref TRANSACTION_TYPE_RESUME_CONTRACT: IntCounter = counter(
        "refiner_tx_type_resume_contract",
        "Number of transactions of type: resume_contract"
    );
    pub static ref TRANSACTION_TYPE_SET_KEY_MANAGER: IntCounter = counter(
        "refiner_tx_type_set_key_manager",
        "Number of transactions of type: set_key_manager"
    );
    pub static ref TRANSACTION_TYPE_ADD_RELAYER_KEY: IntCounter = counter(
        "refiner_tx_type_add_relayer_key",
        "Number of transactions of type: add_relayer_key"
    );
    pub static ref TRANSACTION_TYPE_STORE_RELAYER_KEY_CALLBACK: IntCounter = counter(
        "refiner_tx_type_store_relayer_key_callback",
        "Number of transactions of type: store_relayer_key_callback"
    );
    pub static ref TRANSACTION_TYPE_REMOVE_RELAYER_KEY: IntCounter = counter(
        "refiner_tx_type_remove_relayer_key",
        "Number of transactions of type: remove_relayer_key"
    );
    pub static ref TRANSACTION_TYPE_START_HASHCHAIN : IntCounter = counter(
        "refiner_tx_type_start_hashchain",
        "Number of transactions of type: start_hashchain"
    );
    pub static ref TRANSACTION_TYPE_SET_ERC20_METADATA : IntCounter = counter(
        "refiner_tx_type_set_erc20_metadata",
        "Number of transactions of type: set_erc20_metadata"
    );
    pub static ref TRANSACTION_TYPE_EXIT_TO_NEAR : IntCounter = counter(
        "refiner_tx_type_exit_to_near",
        "Number of transactions of type: exit_to_near"
    );
    pub static ref TRANSACTION_TYPE_SET_FIXED_GAS : IntCounter = counter(
        "refiner_tx_type_set_fixed_gas",
        "Number of transactions of type: set_fixed_gas"
    );
    pub static ref TRANSACTION_TYPE_SET_SILO_PARAMS : IntCounter = counter(
        "refiner_tx_type_set_silo_params",
        "Number of transactions of type: set_silo_params"
    );
    pub static ref TRANSACTION_TYPE_SET_ETH_CONNECTOR_CONTRACT_ACCOUNT : IntCounter = counter(
        "refiner_tx_type_set_eth_connector_contract_account",
        "Number of transactions of type: set_eth_connector_contract_account"
    );
    pub static ref TRANSACTION_TYPE_REMOVE_ENTRY_FROM_WHITE_LIST: IntCounter = counter(
        "refiner_tx_type_remove_entry_from_white_list",
        "Number of transactions of type: remove_entry_from_white_list"
    );
    pub static ref TRANSACTION_TYPE_ADD_ENTRY_TO_WHITELIST_BATCH: IntCounter = counter(
        "refiner_tx_type_add_entry_to_whitelist_batch",
        "Number of transactions of type: add_entry_to_whitelist_batch"
    );
    pub static ref TRANSACTION_TYPE_ADD_ENTRY_TO_WHITELIST: IntCounter = counter(
        "refiner_tx_type_add_entry_to_whitelist",
        "Number of transactions of type: add_entry_to_whitelist"
    );
    pub static ref TRANSACTION_TYPE_SET_WHITELIST_STATUS: IntCounter = counter(
        "refiner_tx_type_set_whitelist_status",
        "Number of transactions of type: set_whitelist_status"
    );
    pub static ref TRANSACTION_TYPE_MIRROR_ERC20_TOKEN_CALLBACK: IntCounter = counter(
        "refiner_tx_type_mirror_erc20_token_callback",
        "Number of transactions of type: mirror_erc20_token_callback"
    );
    pub static ref TRANSACTION_TYPE_WITHDRAW_WNEAR_TO_ROUTER: IntCounter = counter(
        "refiner_tx_type_withdraw_wnear_to_router",
        "Number of transactions of type: withdraw_wnear_to_router"
    );
}

pub fn record_metric(tx_kind: &TransactionKindTag) {
    match tx_kind {
        TransactionKindTag::Submit => {
            TRANSACTION_TYPE_SUBMIT.inc();
        }
        TransactionKindTag::SubmitWithArgs => {
            TRANSACTION_TYPE_SUBMIT_WITH_ARGS.inc();
        }
        TransactionKindTag::Call => {
            TRANSACTION_TYPE_CALL.inc();
        }
        TransactionKindTag::PausePrecompiles => {
            TRANSACTION_TYPE_PAUSE_PRECOMPILES.inc();
        }
        TransactionKindTag::ResumePrecompiles => {
            TRANSACTION_TYPE_RESUME_PRECOMPILES.inc();
        }
        TransactionKindTag::SetOwner => {
            TRANSACTION_TYPE_SET_OWNER.inc();
        }
        TransactionKindTag::Deploy => {
            TRANSACTION_TYPE_DEPLOY_CODE.inc();
        }
        TransactionKindTag::DeployErc20 => {
            TRANSACTION_TYPE_DEPLOY_ERC20_TOKEN.inc();
        }
        TransactionKindTag::FtOnTransfer => {
            TRANSACTION_TYPE_FT_ON_TRANSFER.inc();
        }
        TransactionKindTag::Deposit => {
            TRANSACTION_TYPE_DEPOSIT.inc();
        }
        TransactionKindTag::FtTransferCall => {
            TRANSACTION_TYPE_FT_TRANSFER_CALL.inc();
        }
        TransactionKindTag::FinishDeposit => {
            TRANSACTION_TYPE_FINISH_DEPOSIT.inc();
        }
        TransactionKindTag::ResolveTransfer => {
            TRANSACTION_TYPE_FT_RESOLVE_TRANSFER.inc();
        }
        TransactionKindTag::FtTransfer => {
            TRANSACTION_TYPE_FT_TRANSFER.inc();
        }
        TransactionKindTag::Withdraw => {
            TRANSACTION_TYPE_WITHDRAW.inc();
        }
        TransactionKindTag::StorageDeposit => {
            TRANSACTION_TYPE_STORAGE_DEPOSIT.inc();
        }
        TransactionKindTag::StorageUnregister => {
            TRANSACTION_TYPE_STORAGE_UNREGISTER.inc();
        }
        TransactionKindTag::StorageWithdraw => {
            TRANSACTION_TYPE_STORAGE_WITHDRAW.inc();
        }
        TransactionKindTag::SetPausedFlags => {
            TRANSACTION_TYPE_SET_PAUSED_FLAGS.inc();
        }
        TransactionKindTag::RegisterRelayer => {
            TRANSACTION_TYPE_REGISTER_RELAYER.inc();
        }
        TransactionKindTag::SetConnectorData => {
            TRANSACTION_TYPE_SET_CONNECTOR_DATA.inc();
        }
        TransactionKindTag::NewConnector => {
            TRANSACTION_TYPE_NEW_CONNECTOR.inc();
        }
        TransactionKindTag::NewEngine => {
            TRANSACTION_TYPE_NEW_ENGINE.inc();
        }
        TransactionKindTag::FactoryUpdate => {
            TRANSACTION_TYPE_FACTORY_UPDATE.inc();
        }
        TransactionKindTag::FactoryUpdateAddressVersion => {
            TRANSACTION_TYPE_FACTORY_UPDATE_ADDRESS_VERSION.inc();
        }
        TransactionKindTag::FactorySetWNearAddress => {
            TRANSACTION_TYPE_FACTORY_SET_WNEAR_ADDRESS.inc();
        }
        TransactionKindTag::SetUpgradeDelayBlocks => {
            TRANSACTION_TYPE_SET_UPGRADE_DELAY_BLOCKS.inc();
        }
        TransactionKindTag::FundXccSubAccount => {
            TRANSACTION_TYPE_FUND_XCC_SUB_ACCOUNT.inc();
        }
        TransactionKindTag::PauseContract => {
            TRANSACTION_TYPE_PAUSE_CONTRACT.inc();
        }
        TransactionKindTag::ResumeContract => TRANSACTION_TYPE_RESUME_CONTRACT.inc(),
        TransactionKindTag::SetKeyManager => {
            TRANSACTION_TYPE_SET_KEY_MANAGER.inc();
        }
        TransactionKindTag::AddRelayerKey => {
            TRANSACTION_TYPE_ADD_RELAYER_KEY.inc();
        }
        TransactionKindTag::StoreRelayerKeyCallback => {
            TRANSACTION_TYPE_STORE_RELAYER_KEY_CALLBACK.inc();
        }
        TransactionKindTag::RemoveRelayerKey => {
            TRANSACTION_TYPE_REMOVE_RELAYER_KEY.inc();
        }
        TransactionKindTag::StartHashchain => {
            TRANSACTION_TYPE_START_HASHCHAIN.inc();
        }
        TransactionKindTag::Unknown => {
            TRANSACTION_TYPE_UNKNOWN.inc();
        }
        TransactionKindTag::SetErc20Metadata => {
            TRANSACTION_TYPE_SET_ERC20_METADATA.inc();
        }
        TransactionKindTag::ExitToNear => {
            TRANSACTION_TYPE_EXIT_TO_NEAR.inc();
        }
        TransactionKindTag::SetFixedGas => {
            TRANSACTION_TYPE_SET_FIXED_GAS.inc();
        }
        TransactionKindTag::SetSiloParams => {
            TRANSACTION_TYPE_SET_SILO_PARAMS.inc();
        }
        TransactionKindTag::SetEthConnectorContractAccount => {
            TRANSACTION_TYPE_SET_ETH_CONNECTOR_CONTRACT_ACCOUNT.inc();
        }
        TransactionKindTag::RemoveEntryFromWhitelist => {
            TRANSACTION_TYPE_REMOVE_ENTRY_FROM_WHITE_LIST.inc();
        }
        TransactionKindTag::AddEntryToWhitelistBatch => {
            TRANSACTION_TYPE_ADD_ENTRY_TO_WHITELIST_BATCH.inc();
        }
        TransactionKindTag::AddEntryToWhitelist => {
            TRANSACTION_TYPE_ADD_ENTRY_TO_WHITELIST.inc();
        }
        TransactionKindTag::SetWhitelistStatus => {
            TRANSACTION_TYPE_SET_WHITELIST_STATUS.inc();
        }
        TransactionKindTag::MirrorErc20TokenCallback => {
            TRANSACTION_TYPE_MIRROR_ERC20_TOKEN_CALLBACK.inc();
        }
        TransactionKindTag::WithdrawWnearToRouter => {
            TRANSACTION_TYPE_WITHDRAW_WNEAR_TO_ROUTER.inc();
        }
    }
}

fn counter(name: &str, help: &str) -> IntCounter {
    register_int_counter!(opts(name, help)).unwrap()
}

fn gauge(name: &str, help: &str) -> IntGauge {
    register_int_gauge!(opts(name, help)).unwrap()
}

fn opts(name: &str, help: &str) -> Opts {
    opts!(
        name,
        help,
        labels! {"version" => env!("CARGO_PKG_VERSION") }
    )
}
