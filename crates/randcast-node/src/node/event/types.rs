use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum Topic {
    NewBlock(usize),
    NewDKGTask,
    RunDKG,
    DKGPhase,
    DKGSuccess,
    DKGPostProcess,
    NewRandomnessTask(usize),
    NewGroupRelayTask,
    NewGroupRelayConfirmationTask(usize),
    ReadyToHandleRandomnessTask(usize),
    ReadyToHandleGroupRelayTask,
    ReadyToHandleGroupRelayConfirmationTask(usize),
    ReadyToFulfillRandomnessTask(usize),
    ReadyToFulfillGroupRelayTask,
    ReadyToFulfillGroupRelayConfirmationTask(usize),
}
