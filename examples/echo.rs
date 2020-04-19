use std::env::var;

use wx_work::server::App;
use wx_work::server::{RecvMessage, SendMessage, Builder, RecvMessageType};
use async_trait::async_trait;
use dotenv::dotenv;

struct MyApp;

#[async_trait]
impl App for MyApp {
    async fn handle(&self, msg: RecvMessage) -> Option<SendMessage> {
        match msg.msg_ty {
            RecvMessageType::Picture(pic) =>
                Some(SendMessage::new_pic(pic.media_id, msg.from_user_name, msg.to_user_name)),
            RecvMessageType::Text(x) =>
                Some(SendMessage::new_text(x, msg.from_user_name, msg.to_user_name)),
            _ =>  None
        }
    }
}

fn main() {
    dotenv().ok();
    env_logger::init();

    let token = var("TOKEN").unwrap();
    let aes_key = var("AES_KEY").unwrap();
    let corp_id = var("CORP_ID").unwrap();

    let server = Builder::new(MyApp, token, aes_key, corp_id).build().unwrap();
    server.run().unwrap();
}