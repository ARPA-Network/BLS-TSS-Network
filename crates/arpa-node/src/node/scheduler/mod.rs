pub mod dynamic;

pub mod fixed;

use std::convert::TryFrom;

use arpa_node_core::{SchedulerError, SchedulerResult};
use async_trait::async_trait;
use futures::Future;
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, Clone, Hash, PartialEq)]
pub enum TaskType {
    Listener(ListenerType),
    Subscriber(SubscriberType),
    RpcServer(RpcServerType),
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TaskType::Listener(l) => std::fmt::Display::fmt(l, f),
            TaskType::Subscriber(s) => std::fmt::Display::fmt(s, f),
            TaskType::RpcServer(r) => std::fmt::Display::fmt(r, f),
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Clone, Serialize, Deserialize)]
pub enum ListenerType {
    Block,
    PreGrouping,
    PostCommitGrouping,
    PostGrouping,
    NewRandomnessTask,
    ReadyToHandleRandomnessTask,
    RandomnessSignatureAggregation,
}

impl TryFrom<i32> for ListenerType {
    type Error = SchedulerError;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(ListenerType::Block),
            1 => Ok(ListenerType::PreGrouping),
            2 => Ok(ListenerType::PostCommitGrouping),
            3 => Ok(ListenerType::PostGrouping),
            4 => Ok(ListenerType::NewRandomnessTask),
            5 => Ok(ListenerType::ReadyToHandleRandomnessTask),
            6 => Ok(ListenerType::RandomnessSignatureAggregation),
            _ => Err(SchedulerError::TaskNotFound),
        }
    }
}

impl std::fmt::Display for ListenerType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ListenerType::Block => write!(f, "Block"),
            ListenerType::PreGrouping => write!(f, "PreGrouping"),
            ListenerType::PostGrouping => write!(f, "PostGrouping"),
            ListenerType::ReadyToHandleRandomnessTask => write!(f, "ReadyToHandleRandomnessTask"),
            ListenerType::RandomnessSignatureAggregation => {
                write!(f, "RandomnessSignatureAggregation")
            }
            ListenerType::PostCommitGrouping => write!(f, "PostCommitGrouping"),
            ListenerType::NewRandomnessTask => write!(f, "NewRandomnessTask"),
        }
    }
}

#[derive(Debug, Eq, Clone, Hash, PartialEq)]
pub enum SubscriberType {
    Block,
    PreGrouping,
    InGrouping,
    PostSuccessGrouping,
    PostGrouping,
    ReadyToHandleRandomnessTask,
    RandomnessSignatureAggregation,
}

impl std::fmt::Display for SubscriberType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SubscriberType::Block => write!(f, "Block"),
            SubscriberType::PreGrouping => write!(f, "PreGrouping"),
            SubscriberType::InGrouping => write!(f, "InGrouping"),
            SubscriberType::PostSuccessGrouping => write!(f, "PostSuccessGrouping"),
            SubscriberType::PostGrouping => write!(f, "PostGrouping"),
            SubscriberType::ReadyToHandleRandomnessTask => write!(f, "ReadyToHandleRandomnessTask"),
            SubscriberType::RandomnessSignatureAggregation => {
                write!(f, "RandomnessSignatureAggregation")
            }
        }
    }
}

#[derive(Debug, Eq, Clone, Hash, PartialEq)]
pub enum RpcServerType {
    Committer,
    Management,
}

impl std::fmt::Display for RpcServerType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RpcServerType::Committer => write!(f, "Committer"),
            RpcServerType::Management => write!(f, "Management"),
        }
    }
}

pub trait TaskScheduler {
    fn add_task<T>(&mut self, task_type: TaskType, future: T) -> SchedulerResult<()>
    where
        T: Future<Output = ()> + Send + 'static,
        T::Output: Send + 'static;
}

#[async_trait]
pub trait FixedTaskScheduler: TaskScheduler {
    async fn join(mut self);

    async fn abort(&mut self, task_type: &TaskType) -> SchedulerResult<()>;

    fn get_tasks(&self) -> Vec<&TaskType>;
}

pub trait DynamicTaskScheduler: TaskScheduler {
    fn add_task_with_shutdown_signal<T, P, F>(
        &mut self,
        future: T,
        shutdown_predicate: P,
        shutdown_check_frequency: u64,
    ) where
        T: Future<Output = ()> + Send + 'static,
        T::Output: Send + 'static,
        P: Fn() -> F + Sync + Send + 'static,
        F: Future<Output = bool> + Send + 'static;
}
