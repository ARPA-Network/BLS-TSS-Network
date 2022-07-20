use thiserror::Error;
use threshold_bls::sig::{BLSError, G1Scheme, ThresholdError};

use dkg_core::{primitives::DKGError, NodeError as DKGNodeError};

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

    #[error("there is no dkg key pair yet")]
    NoDKGKeyPair,

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

    #[error("there is no signature cache yet")]
    CommitterCacheNotExisted,

    #[error("the node is not the committer of the group")]
    NotCommitter,

    #[error("there is no task yet")]
    NoTaskAvailable,

    #[error("the chain id is not supported in the group")]
    InvalidChainId,

    #[error("the message of the task is different from the committer")]
    InvalidTaskMessage,

    #[error("There is already this chain id in the context. Please check config.yml")]
    RepeatedChainId,
}
