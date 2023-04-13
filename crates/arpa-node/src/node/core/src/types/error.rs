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
}
