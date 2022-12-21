use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum BLSTaskError {
    #[error("task not found")]
    TaskNotFound,

    #[error("there is no signature cache yet")]
    CommitterCacheNotExisted,
}

pub type SchedulerResult<A> = Result<A, SchedulerError>;

#[derive(Debug, Error, PartialEq)]
pub enum SchedulerError {
    #[error("task not found")]
    TaskNotFound,

    #[error("task already exists")]
    TaskAlreadyExisted,
}
