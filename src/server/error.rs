use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum MessageError {
    #[error("message invalid signature")]
    InvalidSignature,
    #[error("message decrypt failed, reason: {0}")]
    DecryptFailed(String),
    #[error("message encrypt failed, reason: {0}")]
    EncryptFailed(String),
    #[error("message parse failed, reason: {0}")]
    ParseFailed(String),
    #[error("message missing field: {0}")]
    MissingField(&'static str),
    #[error("message invalid field type : {0}")]
    InvalidFieldType(String),
    #[error("message invalid message type: {0}")]
    InvalidMessageType(String),
}

pub(crate) type Result<T> = std::result::Result<T, MessageError>;
