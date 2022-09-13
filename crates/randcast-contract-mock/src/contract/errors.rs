use thiserror::Error;
use threshold_bls::sig::BLSError;

pub type ControllerResult<A> = Result<A, ControllerError>;

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("this is only for admin")]
    AuthenticationFailed,

    #[error("there is no task yet")]
    NoTaskAvailable,

    #[error("signature task not found in list of pending_signature_tasks")]
    TaskNotFound,

    #[error("signature task is still exclusive for the assigned group")]
    TaskStillExclusive,

    #[error("relay task not found in list of relayed_group_cache")]
    RelayTaskNotFound,

    #[error("relayed_group_epoch is smaller or equal to the latest: {0}")]
    RelayGroupDataObsolete(usize),

    #[error("confirmed group_cache is different from relayed_group_cache")]
    RelayGroupDataInconsistency,

    #[error("mismatch between current global epoch and relayed relay_task_index")]
    RelayFulfillmentNotInOrder,

    #[error("there is already an in-progress relayed_group_cache")]
    RelayFulfillmentRepeated,

    #[error("the relay confirmation task has not expired yet")]
    RelayConfirmationTaskStillAvailable,

    #[error("signature task can only be fulfilled by the committer")]
    NotFromCommitter,

    #[error("the member: {0} is not contained in the group: {1}")]
    MemberNotExisted(String, usize),

    #[error("the group index is not exist")]
    GroupNotExisted,

    #[error("the group is ready to work")]
    GroupActivated,

    #[error("there is already an initial group")]
    InitialGroupExisted,

    #[error("the coordinator with the group index: {0} is not exist")]
    CoordinatorNotExisted(usize),

    #[error("the coordinator has not ended yet")]
    CoordinatorNotEnded,

    #[error("the output can't be submitted since DKG has already ended")]
    CoordinatorEnded,

    #[error("the coordinator epoch is different from the latest: {0}")]
    CoordinatorEpochObsolete(usize),

    #[error("the node is not registered")]
    NodeNotExisted,

    #[error("the node has already registered")]
    NodeExisted,

    #[error("the node is ready to work")]
    NodeActivated,

    #[error("the node is pending until block height #{0}")]
    NodeNotAvailable(usize),

    #[error("the reward record of the address is not exist")]
    RewardRecordNotExisted,

    #[error("the group epoch is different from the latest: {0}")]
    GroupEpochObsolete(usize),

    #[error("you have already committed the dkg output")]
    CommitCacheExisted,

    #[error("there is pending verifiable_signature_reward related to the node as the committer")]
    VerifiableSignatureRewardAsCommitterExisted,

    #[error("the verifiable_signature_reward is not exist")]
    VerifiableSignatureRewardNotExisted,

    #[error("the verifiable_signature_reward has been verified successfully")]
    SignatureRewardVerifiedSuccessfully,

    #[error("deserialization failed: the public key is not a valid G1 point {0})")]
    PublicKeyBadFormat(#[from] bincode::Error),

    #[error("BLS verify failed")]
    BLSVerifyFailed(#[from] BLSError),

    #[error(transparent)]
    CoordinatorError(#[from] CoordinatorError),

    #[error("the participant is not in the specified group")]
    ParticipantNotExisted,

    #[error("there is no valid group to generate randomness for now")]
    NoVaildGroup,
}

pub type CoordinatorResult<A> = Result<A, CoordinatorError>;

#[derive(Debug, Error)]
pub enum CoordinatorError {
    #[error("you are not allowlisted!")]
    NotAllowlisted,

    #[error("you have already been allowlisted!")]
    AlreadyAllowlisted,

    #[error("you are not registered!")]
    NotRegistered,

    #[error("you have already registered!")]
    AlreadyRegistered,

    #[error("DKG has already started")]
    AlreadyStarted,

    #[error("DKG has already ended")]
    DKGEnded,

    #[error("DKG publish data is out of phase")]
    DKGPublishEnded,

    #[error("you already published your shares")]
    SharesExisted,

    #[error("you already published your responses")]
    ResponsesExisted,

    #[error("you already published your justifications")]
    JustificationsExisted,
}
