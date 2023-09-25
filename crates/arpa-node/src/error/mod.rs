use arpa_contract_client::error::ContractClientError;
use arpa_core::SchedulerError;
use arpa_dal::error::DataAccessError;
use arpa_sqlite_db::DBError;
use dkg_core::{primitives::DKGError, NodeError as DKGNodeError};
use ethers::providers::ProviderError;
use rustc_hex::FromHexError;
use thiserror::Error;
use threshold_bls::sig::BLSError;

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

    #[error("the node is not the committer of the group")]
    NotCommitter,

    #[error("the block_number: {0} is invalid")]
    InvalidBlockNumber(usize),

    #[error("the message of the task is different from the committer")]
    InvalidTaskMessage,

    #[error("not supported task type")]
    InvalidTaskType,

    #[error("There is already this chain id in the context. Please check config.yml")]
    RepeatedChainId,

    #[error("can't connect to the rpc server, please check the endpoint. Original error: {0}")]
    RpcNotAvailableError(tonic::transport::Error),

    #[error(transparent)]
    DataAccessError(#[from] DataAccessError),

    #[error(transparent)]
    SchedulerError(#[from] SchedulerError),

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

    #[error("DKG has not started yet")]
    DKGNotStarted,

    #[error("DKG has ended")]
    DKGEnded,
}
