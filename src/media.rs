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
    errcode: u64,
    errmsg: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub media_id: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadImageResponse {
    errcode: u64,
    errmsg: String,
    pub url: String,
}
