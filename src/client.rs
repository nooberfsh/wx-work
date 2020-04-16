use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::runtime::{Builder, Runtime};
use tokio::time::{delay_for, Duration};

use crate::{Error, Result};

pub struct Client {
    access_token: Arc<RwLock<String>>,
    http_client: Arc<reqwest::Client>,
    refresh_token_thread: JoinHandle<()>, // TODO: should join handle when drop ?
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
    pub fn new(corp_id: String, corp_secret: String, get_token_url: String) -> Result<Self> {
        let url = format!(
            "{}?corpid={}&corpsecret={}",
            get_token_url, corp_id, corp_secret
        );

        let mut runtime = Builder::new()
            .enable_all()
            .threaded_scheduler()
            .build()
            .unwrap();

        let http_client = Arc::new(reqwest::Client::new());
        let resp = runtime.block_on(get_access_token(&http_client, &url))?;

        let access_token = Arc::new(RwLock::new(resp.access_token));

        let refresh_token_thread = start_refresh_token_thread(
            runtime,
            http_client.clone(),
            url,
            resp.expires_in,
            access_token.clone(),
        );

        let ret = Client {
            access_token,
            http_client,
            refresh_token_thread,
        };

        Ok(ret)
    }
}
