use std::env::var;

use async_trait::async_trait;
use dotenv::dotenv;
use wx_work::server::App;
use wx_work::server::{Builder, RecvMessage, RecvMessageType, SendMessage};

struct MyApp;

#[async_trait]
impl App for MyApp {
    async fn handle(&self, msg: RecvMessage) -> Option<SendMessage> {
        match msg.msg_ty {
            RecvMessageType::Picture(pic) => Some(SendMessage::new_pic(
                pic.media_id,
                msg.from_user_name,
                msg.to_user_name,
            )),
            RecvMessageType::Text(x) => Some(SendMessage::new_text(
                x,
                msg.from_user_name,
                msg.to_user_name,
            )),
            _ => None,
        }
    }
}

fn main() {
    dotenv().ok();
    env_logger::init();

    let token = var("TOKEN").unwrap();
    let aes_key = var("AES_KEY").unwrap();

    let server = Builder::new(MyApp, token, aes_key).build().unwrap();
    server.run().unwrap();
}
