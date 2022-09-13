use std::env::VarError;

use super::contract_client::ethers::WalletSigner;
use crate::node::dal::sqlite::DBError;
use dkg_core::{primitives::DKGError, NodeError as DKGNodeError};
use ethers::providers::Http as HttpProvider;
use ethers::signers::WalletError;
use ethers::{
    prelude::{signer::SignerMiddlewareError, ContractError, ProviderError},
    providers::Provider,
    signers::LocalWallet,
};
use rustc_hex::FromHexError;
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

    #[error(transparent)]
    RpcClientError(#[from] tonic::transport::Error),

    #[error(transparent)]
    RpcResponseError(#[from] tonic::Status),

    #[error("there is no signature cache yet")]
    CommitterCacheNotExisted,

    #[error("the node is not the committer of the group")]
    NotCommitter,

    #[error("there is no task yet")]
    NoTaskAvailable,

    #[error("randomness task not found")]
    TaskNotFound,

    #[error("the chain id: {0} is not supported in the group")]
    InvalidChainId(usize),

    #[error("the message of the task is different from the committer")]
    InvalidTaskMessage,

    #[error("There is already this chain id in the context. Please check config.yml")]
    RepeatedChainId,

    #[error(transparent)]
    NodeInfoError(#[from] NodeInfoError),

    #[error(transparent)]
    GroupError(#[from] GroupError),

    #[error(transparent)]
    DBError(#[from] DBError),

    #[error(transparent)]
    ContractClientError(#[from] ContractClientError),

    #[error(transparent)]
    FromHexError(#[from] FromHexError),

    #[error(transparent)]
    ProviderError(#[from] ProviderError),

    #[error("can't parse address format")]
    AddressFormatError,
}

#[derive(Debug, Error, PartialEq)]
pub enum GroupError {
    #[error("there is no group task yet")]
    NoGroupTask,

    #[error("the group is not exist")]
    GroupNotExisted,

    #[error("you are not contained in the group")]
    MemberNotExisted,

    #[error("there is not an available DKG output")]
    GroupNotReady,

    #[error("there is already an available DKG setup")]
    GroupAlreadyReady,

    #[error("the group index is different from the latest: {0}")]
    GroupIndexObsolete(usize),

    #[error("the group epoch is different from the latest: {0}")]
    GroupEpochObsolete(usize),

    #[error("the group is still waiting for other's DKGOutput to commit")]
    GroupWaitingForConsensus,
}

#[derive(Debug, Error, PartialEq)]
pub enum NodeInfoError {
    #[error("there is no node record yet, please run node with new-run mode")]
    NoNodeRecord,

    #[error("there is no rpc endpoint yet")]
    NoRpcEndpoint,

    #[error("there is no dkg key pair yet")]
    NoDKGKeyPair,
}

#[derive(Debug, Error)]
pub enum ContractClientError {
    #[error(transparent)]
    ChainProviderError(#[from] ProviderError),
    #[error(transparent)]
    ContractError(#[from] ContractError<WalletSigner>),
    #[error(transparent)]
    SignerError(#[from] SignerMiddlewareError<Provider<HttpProvider>, LocalWallet>),
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
