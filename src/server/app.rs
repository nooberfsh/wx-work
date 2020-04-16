use async_trait::async_trait;

use super::{RecvMessage, SendMessage};

#[async_trait]
pub trait App: Send + Sync + 'static {
    async fn handle(&self, msg: RecvMessage) -> Option<SendMessage>;
}
