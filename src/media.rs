use serde::Deserialize;

pub enum FileType {
    Image,
    Voice,
    Video,
    File,
}

impl FileType {
    pub(crate) fn type_desc(&self) -> &'static str {
        use FileType::*;
        match self {
            Image => "image",
            Voice => "voice",
            Video => "video",
            File => "file",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UploadFileResponse {
    pub(crate) errcode: u64,
    pub(crate) errmsg: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub ty: String,
    #[serde(default)]
    pub media_id: String,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadImageResponse {
    pub(crate) errcode: u64,
    pub(crate) errmsg: String,
    #[serde(default)]
    pub url: String,
}
