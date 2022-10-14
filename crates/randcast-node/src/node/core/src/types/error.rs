use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum TaskError {
    #[error("task not found")]
    TaskNotFound,

    #[error("there is no signature cache yet")]
    CommitterCacheNotExisted,
}
