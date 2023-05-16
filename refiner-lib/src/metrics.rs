use aurora_standalone_engine::types::InnerTransactionKind;
use lazy_static::lazy_static;
use prometheus::{self, register_int_counter, register_int_gauge, IntCounter, IntGauge};

lazy_static! {
    pub static ref MISSING_SHARDS: IntCounter =
        register_int_counter!("refiner_missing_shards", "Blocks that are missing shards.").unwrap();
    pub static ref BATCHED_ACTIONS: IntCounter = register_int_counter!(
        "refiner_batched_actions",
        "Transactions that uses batched actions"
    )
    .unwrap();
    pub static ref ERROR_BUILDING_TRANSACTION: IntCounter = register_int_counter!(
        "refiner_error_building_transaction",
        "Error building transaction"
    )
    .unwrap();
    pub static ref LATEST_BLOCK_PROCESSED: IntGauge = register_int_gauge!(
        "refiner_number_of_latest_block_processed",
        "Height of last block processed. Can be slightly out of sync with the actual height given multiple process."
    )
    .unwrap();
    pub static ref FAILING_NEAR_TRANSACTION: IntCounter =
        register_int_counter!("refiner_near_transaction_failed", "NEAR Transaction failed")
            .unwrap();
    pub static ref TRANSACTIONS: IntCounter = register_int_counter!(
        "refiner_transactions",
        "Number of transactions after filter"
    )
    .unwrap();
    pub static ref TRANSACTIONS_ACTION: IntCounter = register_int_counter!(
        "refiner_transaction_actions",
        "Number of actions inside transactions"
    )
    .unwrap();
    pub static ref TRANSACTIONS_DATA: IntCounter = register_int_counter!(
        "refiner_transaction_data",
        "Number of receipts that are of type data"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_SUBMIT: IntCounter = register_int_counter!(
        "refiner_tx_type_submit",
        "Number of transactions of type: submit"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_SUBMIT_WITH_ARGS: IntCounter = register_int_counter!(
        "refiner_tx_type_submit_with_args",
        "Number of transactions of type: submit_with_args"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_CALL: IntCounter = register_int_counter!(
        "refiner_tx_type_call",
        "Number of transactions of type: call"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_PAUSE_PRECOMPILES: IntCounter = register_int_counter!(
        "refiner_tx_type_pause_precompiles",
        "Number of transactions of type: pause_precompiles"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_RESUME_PRECOMPILES: IntCounter = register_int_counter!(
        "refiner_tx_type_resume_precompiles",
        "Number of transactions of type: resume_precompiles"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_SET_OWNER: IntCounter = register_int_counter!(
        "refiner_tx_type_set_owner",
        "Number of transactions of type: set_owner"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_DEPLOY_CODE: IntCounter = register_int_counter!(
        "refiner_tx_type_deploy_code",
        "Number of transactions of type: deploy_code"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_DEPLOY_ERC20_TOKEN: IntCounter = register_int_counter!(
        "refiner_tx_type_deploy_erc20_token",
        "Number of transactions of type: deploy_erc20_token"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_FT_ON_TRANSFER: IntCounter = register_int_counter!(
        "refiner_tx_type_ft_on_transfer",
        "Number of transactions of type: ft_on_transfer"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_DEPOSIT: IntCounter = register_int_counter!(
        "refiner_tx_type_deposit",
        "Number of transactions of type: deposit"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_WITHDRAW: IntCounter = register_int_counter!(
        "refiner_tx_type_withdraw",
        "Number of transactions of type: withdraw"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_FINISH_DEPOSIT: IntCounter = register_int_counter!(
        "refiner_tx_type_finish_deposit",
        "Number of transactions of type: finish_deposit"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_FT_TRANSFER: IntCounter = register_int_counter!(
        "refiner_tx_type_ft_transfer",
        "Number of transactions of type: ft_deposit"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_FT_TRANSFER_CALL: IntCounter = register_int_counter!(
        "refiner_tx_type_ft_transfer_call",
        "Number of transactions of type: ft_transfer_call"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_FT_RESOLVE_TRANSFER: IntCounter = register_int_counter!(
        "refiner_tx_type_ft_resolve_transfer",
        "Number of transactions of type: ft_resolve_transfer"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_STORAGE_DEPOSIT: IntCounter = register_int_counter!(
        "refiner_tx_type_storage_deposit",
        "Number of transactions of type: storage_deposit"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_STORAGE_UNREGISTER: IntCounter = register_int_counter!(
        "refiner_tx_type_storage_unregister",
        "Number of transactions of type: storage_unregister"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_STORAGE_WITHDRAW: IntCounter = register_int_counter!(
        "refiner_tx_type_storage_deposit",
        "Number of transactions of type: storage_deposit"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_SET_PAUSED_FLAGS: IntCounter = register_int_counter!(
        "refiner_tx_type_set_paused_flags",
        "Number of transactions of type: set_paused_flags"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_REGISTER_RELAYER: IntCounter = register_int_counter!(
        "refiner_tx_type_register_relayer",
        "Number of transactions of type: register_relayer"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_REFUND_ON_ERROR: IntCounter = register_int_counter!(
        "refiner_tx_type_refund_on_error",
        "Number of transactions of type: refund_on_error"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_SET_CONNECTOR_DATA: IntCounter = register_int_counter!(
        "refiner_tx_type_set_connector_data",
        "Number of transactions of type: set_connector_data"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_NEW_CONNECTOR: IntCounter = register_int_counter!(
        "refiner_tx_type_new_connector",
        "Number of transactions of type: new_connector"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_NEW_ENGINE: IntCounter = register_int_counter!(
        "refiner_tx_type_new_engine",
        "Number of transactions of type: new_engine"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_SET_UPGRADE_DELAY_BLOCKS: IntCounter = register_int_counter!(
        "refiner_tx_type_set_upgrade_delay_blocks",
        "Number of transactions of type: set_upgrade_delay_blocks"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_FUND_XCC_SUB_ACCOUNT: IntCounter = register_int_counter!(
        "refiner_tx_type_fund_xcc_sub_account",
        "Number of transactions of type: fund_xcc_sub_account"
    )
    .unwrap();
    pub static ref TRANSACTION_TYPE_UNKNOWN: IntCounter = register_int_counter!(
        "refiner_tx_type_unknown",
        "Number of transactions of type: unknown"
    )
    .unwrap();
    pub static ref SKIP_BLOCKS: IntCounter = register_int_counter!(
        "refiner_near_listener_skip_blocks",
        "Number of skip blocks seen"
    )
    .unwrap();
    pub static ref PROCESSED_BLOCKS: IntCounter = register_int_counter!(
        "refiner_near_listener_processed_blocks",
        "Number of blocks processed"
    )
    .unwrap();
    pub static ref UNKNOWN_TX_FOR_RECEIPT: IntCounter = register_int_counter!(
        "refiner_unknown_tx_for_receipt",
        "Number of receipts where the transaction provenance was not known (should be 0)."
    )
    .unwrap();

    pub static ref TRANSACTION_TYPE_FACTORY_UPDATE: IntCounter = register_int_counter!(
                "refiner_tx_type_factory_update",
                        "Number of transactions of type: factory_update"
                            ).unwrap();

        pub static ref TRANSACTION_TYPE_FACTORY_UPDATE_ADDRESS_VERSION: IntCounter = register_int_counter!(
                    "refiner_tx_type_factory_update_address_version",
                            "Number of transactions of type: factory_update_address_version"
                                ).unwrap();

            pub static ref TRANSACTION_TYPE_FACTORY_SET_WNEAR_ADDRESS: IntCounter = register_int_counter!(
                        "refiner_tx_type_factory_set_wnear_address",
                                "Number of transactions of type: factory_set_wnear_address"
                                    ).unwrap();
}

pub(crate) fn record_metric(tx_kind: &InnerTransactionKind) {
    match tx_kind {
        InnerTransactionKind::Submit => {
            TRANSACTION_TYPE_SUBMIT.inc();
        }
        InnerTransactionKind::SubmitWithArgs => {
            TRANSACTION_TYPE_SUBMIT_WITH_ARGS.inc();
        }
        InnerTransactionKind::Call => {
            TRANSACTION_TYPE_CALL.inc();
        }
        InnerTransactionKind::PausePrecompiles => {
            TRANSACTION_TYPE_PAUSE_PRECOMPILES.inc();
        }
        InnerTransactionKind::ResumePrecompiles => {
            TRANSACTION_TYPE_RESUME_PRECOMPILES.inc();
        }
        InnerTransactionKind::SetOwner => {
            TRANSACTION_TYPE_SET_OWNER.inc();
        }
        InnerTransactionKind::Deploy => {
            TRANSACTION_TYPE_DEPLOY_CODE.inc();
        }
        InnerTransactionKind::DeployErc20 => {
            TRANSACTION_TYPE_DEPLOY_ERC20_TOKEN.inc();
        }
        InnerTransactionKind::FtOnTransfer => {
            TRANSACTION_TYPE_FT_ON_TRANSFER.inc();
        }
        InnerTransactionKind::Deposit => {
            TRANSACTION_TYPE_DEPOSIT.inc();
        }
        InnerTransactionKind::FtTransferCall => {
            TRANSACTION_TYPE_FT_TRANSFER_CALL.inc();
        }
        InnerTransactionKind::FinishDeposit => {
            TRANSACTION_TYPE_FINISH_DEPOSIT.inc();
        }
        InnerTransactionKind::ResolveTransfer => {
            TRANSACTION_TYPE_FT_RESOLVE_TRANSFER.inc();
        }
        InnerTransactionKind::FtTransfer => {
            TRANSACTION_TYPE_FT_TRANSFER.inc();
        }
        InnerTransactionKind::Withdraw => {
            TRANSACTION_TYPE_WITHDRAW.inc();
        }
        InnerTransactionKind::StorageDeposit => {
            TRANSACTION_TYPE_STORAGE_DEPOSIT.inc();
        }
        InnerTransactionKind::StorageUnregister => {
            TRANSACTION_TYPE_STORAGE_UNREGISTER.inc();
        }
        InnerTransactionKind::StorageWithdraw => {
            TRANSACTION_TYPE_STORAGE_WITHDRAW.inc();
        }
        InnerTransactionKind::SetPausedFlags => {
            TRANSACTION_TYPE_SET_PAUSED_FLAGS.inc();
        }
        InnerTransactionKind::RegisterRelayer => {
            TRANSACTION_TYPE_REGISTER_RELAYER.inc();
        }
        InnerTransactionKind::RefundOnError => {
            TRANSACTION_TYPE_REFUND_ON_ERROR.inc();
        }
        InnerTransactionKind::SetConnectorData => {
            TRANSACTION_TYPE_SET_CONNECTOR_DATA.inc();
        }
        InnerTransactionKind::NewConnector => {
            TRANSACTION_TYPE_NEW_CONNECTOR.inc();
        }
        InnerTransactionKind::NewEngine => {
            TRANSACTION_TYPE_NEW_ENGINE.inc();
        }
        InnerTransactionKind::FactoryUpdate => {
            TRANSACTION_TYPE_FACTORY_UPDATE.inc();
        }
        InnerTransactionKind::FactoryUpdateAddressVersion => {
            TRANSACTION_TYPE_FACTORY_UPDATE_ADDRESS_VERSION.inc();
        }
        InnerTransactionKind::FactorySetWNearAddress => {
            TRANSACTION_TYPE_FACTORY_SET_WNEAR_ADDRESS.inc();
        }
        InnerTransactionKind::SetUpgradeDelayBlocks => {
            TRANSACTION_TYPE_SET_UPGRADE_DELAY_BLOCKS.inc();
        }
        InnerTransactionKind::FundXccSubAccound => {
            TRANSACTION_TYPE_FUND_XCC_SUB_ACCOUNT.inc();
        }
        InnerTransactionKind::Unknown => {
            TRANSACTION_TYPE_UNKNOWN.inc();
        }
    }
}
