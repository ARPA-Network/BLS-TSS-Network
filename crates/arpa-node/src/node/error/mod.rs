use arpa_node_contract_client::error::ContractClientError;
use arpa_node_dal::error::DataAccessError;
use arpa_node_sqlite_db::DBError;
use dkg_core::{primitives::DKGError, NodeError as DKGNodeError};
use ethers::{providers::ProviderError, signers::WalletError};
use rustc_hex::FromHexError;
use std::env::VarError;
use thiserror::Error;
use threshold_bls::sig::{BLSError, G1Scheme, ThresholdError};

pub type NodeResult<A> = Result<A, NodeError>;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("could not serialize: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("could not deserialize: {0}")]
    DeserializationError(#[from] bincode::Error),

    #[error(transparent)]
    DKGNodeError(#[from] DKGNodeError),

    #[error(transparent)]
    DKGError(#[from] DKGError),

    #[error(transparent)]
    BLSError(#[from] BLSError),

    #[error(transparent)]
    ThresholdError(#[from] ThresholdError<G1Scheme<threshold_bls::curve::bls12381::PairingCurve>>),

    #[error("the node is not the committer of the group")]
    NotCommitter,

    #[error("the chain id: {0} is not supported in the group")]
    InvalidChainId(usize),

    #[error("the message of the task is different from the committer")]
    InvalidTaskMessage,

    #[error("There is already this chain id in the context. Please check config.yml")]
    RepeatedChainId,

    #[error(transparent)]
    DataAccessError(#[from] DataAccessError),

    #[error(transparent)]
    ContractClientError(#[from] ContractClientError),

    #[error(transparent)]
    RpcClientError(#[from] tonic::transport::Error),

    #[error(transparent)]
    RpcResponseError(#[from] tonic::Status),

    #[error(transparent)]
    DBError(#[from] DBError),

    #[error(transparent)]
    ClientBuildError(#[from] anyhow::Error),

    #[error(transparent)]
    FromHexError(#[from] FromHexError),

    #[error(transparent)]
    ProviderError(#[from] ProviderError),

    #[error("can't parse address format")]
    AddressFormatError,

    #[error("there is not an available DKG output")]
    GroupNotReady,

    #[error("you are not contained in the group")]
    MemberNotExisted,
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
