use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0} not found")]
    NotFound(String),
    #[error("invalid input: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
