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

    #[error("Invalid URL error: {0}")]
    InvalidUrl(String),

    #[error("HTTP request error: {0}")]
    HttpRequestError(String),

    #[error("JSON error: {0}")]
    JsonError(String),

    #[error("Invalid session key length")]
    SessionKeyError,

    #[error("Environment variable error: {0}")]
    EnvironmentVariableError(String),

    #[error("Job error: {0}")]
    JobError(String),

    #[error("Command error: {0}")]
    CommandError(String),

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(String),

    #[error("IO error: {0}")]
    IoError(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::OtherError(s)
    }
}
