use thiserror::Error;

/// Represents errors that can occur in the Tangle AVS
#[derive(Debug, Error)]
pub enum Error {
    #[error("EigenLayer registration error: {0}")]
    EigenLayerRegistration(String),

    #[error("Tangle registration error: {0}")]
    TangleRegistration(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}
