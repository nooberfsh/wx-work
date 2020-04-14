use async_trait::async_trait;

use crate::{SendMessage, RecvMessage};

#[async_trait]
trait App {
    async fn handle(&self, msg: RecvMessage) -> Option<SendMessage>;
}
