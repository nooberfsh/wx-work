use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{web, App as ActixApp, Error, HttpResponse, HttpServer};
use futures::StreamExt;
use log::{info, warn};
use serde::Deserialize;

use super::crypto::Crypto;
use super::{App, RecvMessage};

pub struct Builder<T: App> {
    app: T,
    token: String,
    encoding_aes_key: String,
    port: Option<u16>, // optional, default is 12349
}

pub struct Server<T: App> {
    app: T,
    crypto: Crypto,
    port: u16,
}

impl<T: App> Builder<T> {
    pub fn new(app: T, token: impl ToString, encoding_aes_key: impl ToString) -> Self {
        Builder {
            app,
            token: token.to_string(),
            encoding_aes_key: encoding_aes_key.to_string(),
            port: None,
        }
    }

    pub fn port(mut self, p: u16) -> Self {
        self.port = Some(p);
        self
    }

    pub fn build(self) -> anyhow::Result<Server<T>> {
        let app = self.app;
        let crypto = Crypto::new(self.token, self.encoding_aes_key)?;
        let port = self.port.unwrap_or(12349);
        let s = Server { app, crypto, port };
        Ok(s)
    }
}

impl<T: App> Server<T> {
    // caller should provide a tokio runtime
    // https://github.com/actix/actix-web/issues/1283
    pub async fn run(self) -> std::io::Result<()> {
        let local = tokio::task::LocalSet::new();
        let sys = actix_web::rt::System::run_in_tokio("server", &local);

        let server = web::Data::new(self);
        let addr = format!("0.0.0.0:{}", server.port);
        HttpServer::new(move || {
            ActixApp::new()
                .app_data(server.clone())
                .route("/", web::get().to(validate::<T>))
                .route("/", web::post().to(recv::<T>))
        })
        .bind(addr)?
        .run()
        .await?;

        sys.await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ValidateParams {
    msg_signature: String,
    timestamp: u64,
    nonce: u64,
    echostr: String,
}

#[derive(Debug, Deserialize)]
struct RecvParams {
    msg_signature: String,
    timestamp: u64,
    nonce: u64,
}

async fn validate<T: App>(
    info: web::Query<ValidateParams>,
    server: web::Data<Server<T>>,
) -> HttpResponse {
    info!("validate request: params: {:?}", info);

    let crypto = &server.crypto;
    let payload = match crypto.decrypt(&info.echostr) {
        Ok(d) => d,
        Err(e) => {
            warn!("decrypt validate message failed, reason: {}", e);
            return HttpResponse::BadRequest().finish();
        }
    };

    HttpResponse::Ok().body(payload.data)
}

async fn recv<T: App>(
    info: web::Query<RecvParams>,
    mut body: web::Payload,
    server: web::Data<Server<T>>,
) -> Result<HttpResponse, Error> {
    info!("receive request: params: {:?}", info);

    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item?);
    }

    let crypto = &server.crypto;
    let msg = match RecvMessage::parse(
        &bytes,
        &crypto,
        info.timestamp,
        info.nonce,
        &info.msg_signature,
    ) {
        Ok(d) => d,
        Err(e) => {
            warn!("parse message failed, reason: {}", e);
            return Ok(HttpResponse::BadRequest().finish());
        }
    };

    match server.app.handle(msg).await {
        Some(m) => {
            let msg = m
                .serialize(current_timestamp(), gen_nonce(), crypto)
                .unwrap();
            Ok(HttpResponse::Ok().body(msg))
        }
        None => Ok(HttpResponse::Ok().finish()),
    }
}

///////////////////////////// helper functions ///////////////////////////////////////////////

#[inline]
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[inline]
fn gen_nonce() -> u64 {
    rand::random()
}
