use arpa_dal::error::DataAccessError;
use arpa_dal::error::GroupError;
use sea_orm::{DatabaseConnection, DbErr};
use std::sync::Arc;
use thiserror::Error;

pub type DBResult<A> = Result<A, DBError>;

#[derive(Debug, Error, PartialEq)]
pub enum DBError {
    #[error(transparent)]
    DbError(#[from] DbErr),
    #[error(transparent)]
    GroupError(#[from] GroupError),
}

impl From<DBError> for DataAccessError {
    fn from(e: DBError) -> Self {
        DataAccessError::DBError(anyhow::Error::from(e))
    }
}

#[derive(Default, Debug, Clone)]
pub struct SqliteDB {
    pub(crate) connection: Arc<DatabaseConnection>,
}
