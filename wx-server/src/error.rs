use base64::DecodeError;
use openssl::error::ErrorStack;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("base64 error: {0}")]
    Base64(#[from] DecodeError),
    #[error("crypto error: {0}")]
    Openssl(#[from] ErrorStack),
    #[error("invalid aes key, length != 43")]
    InvalidAesKey,
    #[error("invalid message")]
    InvalidMessage,
    #[error("parsing message failed, reason: {0}")]
    MessageParseFailed(String),
    #[error("message missing field: {0}")]
    MessageMissingField(&'static str),
    #[error("message invalid field type : {0}")]
    MessageInvalidFieldType(String),
    #[error("message invalid message type: {0}")]
    MessageInvalidMessageType(String),
}

pub type Result<T> = std::result::Result<T, Error>;
