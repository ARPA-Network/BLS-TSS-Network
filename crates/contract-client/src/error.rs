use alloy::{
    providers::PendingTransactionError, rpc::types::TransactionReceipt, transports::TransportError,
};
use rustc_hex::FromHexError;
use thiserror::Error;

pub type ContractClientResult<A> = Result<A, ContractClientError>;

#[derive(Debug, Error)]
pub enum ContractClientError {
    #[error("custom error: {0}")]
    CustomError(String),
    #[error(transparent)]
    RpcClientError(#[from] tonic::transport::Error),
    #[error(transparent)]
    RpcResponseError(#[from] tonic::Status),
    #[error(transparent)]
    ContractError(#[from] alloy::contract::Error),
    #[error(transparent)]
    SignerError(#[from] alloy::signers::Error),
    #[error(transparent)]
    PendingTransactionError(#[from] PendingTransactionError),
    #[error(transparent)]
    TransportError(#[from] TransportError),
    #[error(transparent)]
    AddressParseError(#[from] FromHexError),
    #[error("can't fetch new block, please check provider")]
    FetchingBlockError,
    #[error("can't fetch dkg task, please check provider")]
    FetchingDkgTaskError,
    #[error("can't fetch randomness task, please check provider")]
    FetchingRandomnessTaskError,
    #[error("can't fetch group relay task, please check provider")]
    FetchingGroupRelayTaskError,
    #[error("can't fetch group relay confirmation task, please check provider")]
    FetchingGroupRelayConfirmationTaskError,
    #[error("there is no task yet")]
    NoTaskAvailable,
    #[error(transparent)]
    HandlingLogSubscriptionError(#[from] anyhow::Error),
    #[error("can't fetch transaction receipt")]
    NoTransactionReceipt,
    #[error("Transaction failed with status equal to 0x0")]
    TransactionFailed(TransactionReceipt),
}
