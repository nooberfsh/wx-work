use std::env::var;

use async_trait::async_trait;
use dotenv::dotenv;
use wx_work::server::{App, SendVideo};
use wx_work::server::{Builder, RecvMessage, RecvMessageType, SendMessage};

struct MyApp;

#[async_trait]
impl App for MyApp {
    async fn handle(&self, msg: RecvMessage) -> Option<SendMessage> {
        match msg.msg_ty {
            RecvMessageType::Picture(p) => Some(SendMessage::new_pic(
                p.media_id,
                msg.from_user_name,
                msg.to_user_name,
            )),
            RecvMessageType::Text(x) => Some(SendMessage::new_text(
                x,
                msg.from_user_name,
                msg.to_user_name,
            )),
            RecvMessageType::Voice(v) => Some(SendMessage::new_voice(
                v.media_id,
                msg.from_user_name,
                msg.to_user_name,
            )),
            RecvMessageType::Video(v) => Some(SendMessage::new_video(
                SendVideo {
                    media_id: v.media_id,
                    title: "hello".into(),
                    description: "world".into(),
                },
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
