use sudachi::config::ConfigError;
use sudachi::error::SudachiError as CoreSudachiError;

#[derive(uniffi::Error, Debug, thiserror::Error)]
pub enum SudachiError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Dictionary error: {0}")]
    Dictionary(String),
    #[error("Tokenization error: {0}")]
    Tokenization(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl SudachiError {
    pub(crate) fn dictionary(err: CoreSudachiError) -> Self {
        Self::Dictionary(err.to_string())
    }

    pub(crate) fn tokenization(err: CoreSudachiError) -> Self {
        Self::Tokenization(err.to_string())
    }

    pub(crate) fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }
}

impl From<ConfigError> for SudachiError {
    fn from(err: ConfigError) -> Self {
        SudachiError::Config(err.to_string())
    }
}
