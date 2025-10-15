use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum Topic {
    NewBlock(u64),
    NewDKGTask,
    RunDKG,
    DKGPhase,
    DKGSuccess,
    DKGPostProcess,
    NewRandomnessTask(u64),
    NewGroupRelayTask,
    NewGroupRelayConfirmationTask(u64),
    ReadyToHandleRandomnessTask(u64),
    ReadyToHandleGroupRelayTask,
    ReadyToHandleGroupRelayConfirmationTask(u64),
    ReadyToFulfillRandomnessTask(u64),
    ReadyToFulfillGroupRelayTask,
    ReadyToFulfillGroupRelayConfirmationTask(u64),
    NodeActivation,
    ProviderReconnection,
}
