use ethers_signers::WalletError;
use std::env::VarError;
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum BLSTaskError {
    #[error("task not found")]
    TaskNotFound,

    #[error("there is no signature cache yet")]
    CommitterCacheNotExisted,

    #[error("already committed partial signature")]
    AlreadyCommittedPartialSignature,

    #[error(transparent)]
    TaskMsgError(#[from] FromUtf8Error),
}

pub type SchedulerResult<A> = Result<A, SchedulerError>;

#[derive(Debug, Error, PartialEq)]
pub enum SchedulerError {
    #[error("task not found")]
    TaskNotFound,

    #[error("task already exists")]
    TaskAlreadyExisted,

    #[error("the chain id: {0} is not supported")]
    InvalidChainId(usize),

    #[error("the listener {1} is not supported in relayed chain {0}")]
    UnsupportedListenerType(usize, String),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("please provide at least a hdwallet, keystore or plain private key(not recommended)")]
    LackOfAccount,
    #[error("bad format")]
    BadFormat,
    #[error(transparent)]
    EnvVarNotExisted(#[from] VarError),
    #[error(transparent)]
    BuildingAccountError(#[from] WalletError),
    #[error("the chain id: {0} is not supported")]
    InvalidChainId(usize),
    #[error("lack of ARPA contract address")]
    LackOfARPAContractAddress,
}
