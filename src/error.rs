use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("http error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("get access token failed, code:{0}, error message: {1}")]
    GetAccessTokenFailed(u64, String),
    #[error("upload file failed, code:{0}, error message: {1}")]
    UploadMediaFailed(u64, String),
}

pub type Result<T> = std::result::Result<T, Error>;
