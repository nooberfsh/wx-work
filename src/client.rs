use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use log::{error, info};
use reqwest::multipart::{Form, Part};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::media::*;
use crate::message::*;
use crate::{Error, Result};

static WX_URL: &str = "https://qyapi.weixin.qq.com";

pub struct Client {
    access_token: Arc<RwLock<String>>,
    http_client: reqwest::Client,
    refresh_token_thread: Option<JoinHandle<()>>,
    is_exit: Arc<AtomicBool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    errcode: u64,
    errmsg: String,
    access_token: String,
    expires_in: u64,
}

fn get_access_token(client: &reqwest::blocking::Client, url: &str) -> Result<AccessTokenResponse> {
    let resp = client.get(url).send()?.json::<AccessTokenResponse>()?;

    if resp.errcode != 0 {
        return Err(Error::GetAccessTokenFailed(resp.errcode, resp.errmsg));
    }

    Ok(resp)
}

fn start_refresh_token_thread(
    url: String,
    access_token: Arc<RwLock<String>>,
    sender: Sender<Result<()>>,
    is_exit: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("wx work client".to_string())
        .spawn(move || {
            let client = reqwest::blocking::Client::new();

            let d = match get_access_token(&client, &url) {
                Ok(d) => d,
                Err(e) => {
                    sender.send(Err(e)).unwrap();
                    return;
                }
            };

            let mut expires_in = d.expires_in;
            {
                let mut token = access_token.write().unwrap();
                *token = d.access_token;
            }
            info!("init token success, expires_in {}", d.expires_in);
            sender.send(Ok(())).unwrap();

            loop {
                //let delay_time = expires_in / 2;
                let delay_time = expires_in / 2;

                thread::park_timeout(Duration::from_secs(delay_time));
                if is_exit.load(Ordering::Acquire) {
                    info!("detect exit signal, exit thread");
                    break;
                }

                match get_access_token(&client, &url) {
                    Ok(d) => {
                        expires_in = d.expires_in;
                        let mut token = access_token.write().unwrap();
                        *token = d.access_token;
                        info!("update token success, expires_in {}", d.expires_in);
                    }
                    Err(e) => error!("refresh token failed, reason: {}", e),
                }
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

        let http_client = reqwest::Client::new();
        let (tx, rx) = mpsc::channel();

        let access_token = Arc::new(RwLock::new("".to_string()));
        let is_exit = Arc::new(AtomicBool::new(false));

        let refresh_token_thread = Some(start_refresh_token_thread(
            url,
            access_token.clone(),
            tx,
            is_exit.clone(),
        ));

        rx.recv().unwrap()?;

        info!("construct Client success");

        let ret = Client {
            access_token,
            http_client,
            refresh_token_thread,
            is_exit,
        };

        Ok(ret)
    }
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

        let ret = self
            .upload_media::<UploadFileResponse>(&url, buf, file_name)
            .await?;
        if ret.errcode != 0 {
            Err(Error::UploadMediaFailed(ret.errcode, ret.errmsg))
        } else {
            Ok(ret)
        }
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

        let ret = self
            .upload_media::<UploadImageResponse>(&url, buf, file_name)
            .await?;
        if ret.errcode != 0 {
            Err(Error::UploadMediaFailed(ret.errcode, ret.errmsg))
        } else {
            Ok(ret)
        }
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

/// 发送应用消息
impl Client {
    pub async fn send_msg(&self, msg: &Message) -> Result<MessageResponse> {
        let url = format!(
            "{}/cgi-bin/message/send?access_token={}",
            WX_URL,
            self.access_token.read().unwrap(),
        );

        let ret = self
            .http_client
            .post(&url)
            .json(&msg)
            .send()
            .await?
            .json()
            .await?;

        Ok(ret)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.is_exit.store(true, Ordering::Release);
        let handle = self.refresh_token_thread.take().unwrap();
        handle.thread().unpark();
        handle
            .join()
            .expect("can not join the refresh token thread");
        info!("join refresh token thread success");
    }
}

// for mannual test
//#[cfg(test)]
//mod tests {
//    use super::*;
//
//    use dotenv::dotenv;
//    use std::env::var;
//
//    #[test]
//    fn test_drop() {
//        dotenv().ok();
//        env_logger::init();
//
//        let corp_id = var("CORP_ID").unwrap();
//        let corp_secret = var("CORP_SECRET").unwrap();
//        let client = Client::new(&corp_id, &corp_secret).unwrap();
//        drop(client);
//    }
//}
