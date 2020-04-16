use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("get access token failed, code:{0}, error message: {1}")]
    GetAccessTokenFailed(u64, String),
    #[error("http error: {0}")]
    HttpError(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
