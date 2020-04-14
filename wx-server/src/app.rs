use async_trait::async_trait;

use crate::{RecvMessage, SendMessage};

#[async_trait]
pub trait App {
    async fn handle(&self, msg: RecvMessage) -> Option<SendMessage>;
}
