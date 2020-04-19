use std::fs::File;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use log::{error, info};
use reqwest::multipart::{Form, Part};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::runtime::{Builder, Runtime};
use tokio::time::{delay_for, Duration};

use crate::{Error, Result};
use std::io::Read;

static WX_URL: &str = "https://qyapi.weixin.qq.com";

pub struct Client {
    access_token: Arc<RwLock<String>>,
    http_client: Arc<reqwest::Client>,
    _refresh_token_thread: JoinHandle<()>, // TODO: should join handle when drop ?
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    errcode: u64,
    errmsg: String,
    access_token: String,
    expires_in: u64,
}

async fn get_access_token(client: &reqwest::Client, url: &str) -> Result<AccessTokenResponse> {
    let resp = client
        .get(url)
        .send()
        .await?
        .json::<AccessTokenResponse>()
        .await?;

    if resp.errcode != 0 {
        return Err(Error::GetAccessTokenFailed(resp.errcode, resp.errmsg));
    }

    Ok(resp)
}

fn start_refresh_token_thread(
    mut runtime: Runtime,
    client: Arc<reqwest::Client>,
    url: String,
    mut expires_in: u64,
    access_token: Arc<RwLock<String>>,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("wx work client".to_string())
        .spawn(move || loop {
            //let delay_time = expires_in / 2;
            let delay_time = expires_in / 2;
            let f = async {
                delay_for(Duration::from_secs(delay_time)).await;
                get_access_token(&client, &url).await
            };

            match runtime.block_on(f) {
                Ok(d) => {
                    expires_in = d.expires_in;
                    let mut token = access_token.write().unwrap();
                    *token = d.access_token;
                    info!("update token success, expires_in {}", d.expires_in);
                }
                Err(e) => error!("refresh token failed, reason: {}", e),
            }
        })
        .unwrap()
}

impl Client {
    pub fn new(corp_id: &str, corp_secret: &str) -> Result<Self> {
        let url = format!(
            "{}/cgi-bin/gettoken?corpid={}&corpsecret={}",
            WX_URL, corp_id, corp_secret
        );

        let mut runtime = Builder::new()
            .enable_all()
            .threaded_scheduler()
            .build()
            .unwrap();

        let http_client = Arc::new(reqwest::Client::new());
        let resp = runtime.block_on(get_access_token(&http_client, &url))?;

        let access_token = Arc::new(RwLock::new(resp.access_token));

        let _refresh_token_thread = start_refresh_token_thread(
            runtime,
            http_client.clone(),
            url,
            resp.expires_in,
            access_token.clone(),
        );

        let ret = Client {
            access_token,
            http_client,
            _refresh_token_thread,
        };

        Ok(ret)
    }
}

pub enum FileType {
    Image,
    Voice,
    Video,
    File,
}

impl FileType {
    fn type_desc(&self) -> &'static str {
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
    ty: String,
    media_id: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadImageResponse {
    errcode: u64,
    errmsg: String,
    url: String,
}

/// 素材管理
impl Client {
    pub async fn upload_file(&self, ty: FileType, path: &str) -> Result<UploadFileResponse> {
        let url = format!(
            "{}/cgi-bin/media/upload?access_token={}&type={}",
            WX_URL,
            self.access_token.read().unwrap(),
            ty.type_desc()
        );

        let mut f = File::open(path)?;
        let file_name = Path::new(path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(); // TODO need handle unwrap
        let mut buf = vec![];
        f.read_to_end(&mut buf)?;

        self.upload_media(&url, buf, file_name).await
    }

    pub async fn upload_image(&self, path: &str) -> Result<UploadImageResponse> {
        let url = format!(
            "{}/cgi-bin/media/uploadimg?access_token={}",
            WX_URL,
            self.access_token.read().unwrap(),
        );

        let mut f = File::open(path)?;
        let file_name = Path::new(path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(); // TODO need handle unwrap
        let mut buf = vec![];
        f.read_to_end(&mut buf)?;

        self.upload_media(&url, buf, file_name).await
    }

    async fn upload_media<T: DeserializeOwned>(
        &self,
        url: &str,
        data: Vec<u8>,
        file_name: String,
    ) -> Result<T> {
        let part = Part::bytes(data).file_name(file_name);
        let form = Form::new().part("media", part);

        let ret = self
            .http_client
            .post(url)
            .multipart(form)
            .send()
            .await?
            .json()
            .await?;

        Ok(ret)
    }
}
