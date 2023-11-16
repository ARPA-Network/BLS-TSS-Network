use arpa_core::BLSTaskError;
use thiserror::Error;

pub type DataAccessResult<A> = Result<A, DataAccessError>;

#[derive(Debug, Error)]
pub enum DataAccessError {
    #[error(transparent)]
    NodeInfoError(#[from] NodeInfoError),

    #[error(transparent)]
    GroupError(#[from] GroupError),

    #[error(transparent)]
    RandomnessTaskError(#[from] RandomnessTaskError),

    #[error(transparent)]
    TaskError(#[from] BLSTaskError),

    #[error(transparent)]
    DBError(anyhow::Error),

    #[error("the chain id: {0} is not supported")]
    InvalidChainId(usize),

    #[error("could not deserialize: {0}")]
    DeserializationError(#[from] bincode::Error),
}

#[derive(Debug, Error, PartialEq)]
pub enum GroupError {
    #[error("there is no group task yet")]
    NoGroupTask,

    #[error("the group is not exist")]
    GroupNotExisted,

    #[error("the member is not exist")]
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
    #[error("there is no rpc endpoint yet")]
    NoRpcEndpoint,

    #[error("there is no dkg key pair yet")]
    NoDKGKeyPair,
}

#[derive(Debug, Error, PartialEq)]
pub enum RandomnessTaskError {
    #[error("there is no randomness task with request id:{0}")]
    NoRandomnessTask(String),
}
