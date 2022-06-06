use engine_standalone_storage::sync::types::TransactionKind;
use strum::EnumString;

#[derive(EnumString)]
pub(crate) enum InnerTransactionKind {
    Submit,
    Call,
    Deploy,
    DeployErc20,
    FtOnTransfer,
    Deposit,
    FtTransferCall,
    FinishDeposit,
    ResolveTransfer,
    FtTransfer,
    Withdraw,
    StorageDeposit,
    StorageUnregister,
    StorageWithdraw,
    SetPausedFlags,
    RegisterRelayer,
    RefundOnError,
    SetConnectorData,
    NewConnector,
    NewEngine,
    Unknown,
}

/// Used to make sure InnerTransactionKind is kept in sync with TransactionKind
impl From<TransactionKind> for InnerTransactionKind {
    fn from(tx: TransactionKind) -> Self {
        match tx {
            TransactionKind::Submit(_) => InnerTransactionKind::Submit,
            TransactionKind::Call(_) => InnerTransactionKind::Call,
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
            TransactionKind::Unknown => InnerTransactionKind::Unknown,
        }
    }
}
