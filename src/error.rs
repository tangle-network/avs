use thiserror::Error;

/// Represents errors that can occur in the Tangle AVS
#[derive(Debug, Error)]
pub enum Error {
    #[error("EigenLayer registration error: {0}")]
    EigenLayerRegistrationError(String),

    #[error("Tangle registration error: {0}")]
    TangleRegistrationError(String),

    #[error("Signer error: {0}")]
    SignerError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Other error: {0}")]
    OtherError(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}
