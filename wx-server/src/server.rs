use actix_web::{
    error, get, post, web, App as ActixApp, Error, HttpResponse, HttpServer, Responder,
};
use futures::{Future, Stream, StreamExt};
use log::{info, warn};
use serde::Deserialize;

use crate::crypto::Crypto;
use crate::{App, RecvMessage};

pub struct Builder<T: App> {
    app: T,
    token: String,
    encoding_aes_key: String,
}

pub struct Server<T: App> {
    inner: Option<ServerInner<T>>,
}

struct ServerInner<T: App> {
    app: T,
    crypto: Crypto,
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

impl<T: App> Builder<T> {
    pub fn new(app: T, token: String, encoding_aes_key: String) -> Self {
        Builder {
            app,
            token,
            encoding_aes_key,
        }
    }

    pub fn build(self) -> crate::Result<Server<T>> {
        let app = self.app;
        let crypto = Crypto::new(self.token, self.encoding_aes_key)?;
        let inner = ServerInner { app, crypto };
        Ok(Server { inner: Some(inner) })
    }
}

impl<T: App> Server<T> {
    #[actix_rt::main]
    pub async fn run(mut self) -> std::io::Result<()> {
        let server = self.inner.take().unwrap();
        let data = web::Data::new(server);
        HttpServer::new(move || {
            ActixApp::new()
                .app_data(data.clone())
                .route("/", web::get().to(validate::<T>))
                .route("/", web::post().to(recv::<T>))
        })
        .bind("0.0.0.0:12349")? // TODO: specify port in builder
        .run()
        .await
    }
}

async fn validate<T: App>(
    info: web::Query<ValidateParams>,
    server: web::Data<ServerInner<T>>,
) -> HttpResponse {
    info!("validate request: params: {:?}", info);

    let crypto = &server.crypto;
    let ret = match crypto.decrypt(&info.echostr) {
        Ok(d) => d,
        Err(e) => {
            warn!("decrypt validate message failed, reason: {}", e);
            return HttpResponse::BadRequest().finish();
        }
    };

    HttpResponse::Ok().body(ret.data)
}

async fn recv<T: App>(
    info: web::Query<RecvParams>,
    mut body: web::Payload,
    server: web::Data<ServerInner<T>>,
) -> Result<HttpResponse, Error> {
    info!("receive request: params: {:?}", info);

    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item?);
    }

    let crypto = &server.crypto;
    let app = &server.app;
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

fn current_timestamp() -> u64 {
    todo!()
}

fn gen_nonce() -> u64 {
    todo!()
}
