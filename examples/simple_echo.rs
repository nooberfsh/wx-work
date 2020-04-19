use async_trait::async_trait;
use wx_work::server::{App, Builder, RecvMessage, RecvMessageType, SendMessage};

struct MyApp;

#[async_trait]
impl App for MyApp {
    async fn handle(&self, msg: RecvMessage) -> Option<SendMessage> {
        match msg.msg_ty {
            RecvMessageType::Text(x) => Some(SendMessage::new_text(x, msg.from_user_name, msg.to_user_name)),
            _ => None,
        }
    }
}

fn main() {
    let token = "";
    let aes_key = "";
    let server = Builder::new(MyApp, token, aes_key).build().unwrap();
    server.run().unwrap();
}
