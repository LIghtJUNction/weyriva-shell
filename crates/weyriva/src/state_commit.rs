use crate::error::Error;

/// Distinguishes failures before and after the atomic rename commit point.
#[derive(Debug, thiserror::Error)]
pub enum StateWriteError {
    #[error("state write failed before commit: {0}")]
    PreCommit(Error),
    #[error("state was committed but durability confirmation failed: {0}")]
    PostCommit(Error),
}

impl StateWriteError {
    #[must_use]
    pub const fn committed(&self) -> bool {
        matches!(self, Self::PostCommit(_))
    }

    #[must_use]
    pub const fn error(&self) -> &Error {
        match self {
            Self::PreCommit(error) | Self::PostCommit(error) => error,
        }
    }

    #[must_use]
    pub fn into_error(self) -> Error {
        match self {
            Self::PreCommit(error) | Self::PostCommit(error) => error,
        }
    }
}

pub type StateWriteResult<T> = std::result::Result<T, StateWriteError>;
