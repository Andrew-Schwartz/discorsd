use async_tungstenite::tungstenite::protocol::CloseFrame;
use async_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use crate::BotState;

impl<B: Send + Sync> BotState<B> {
    pub async fn log_out(&self) {
        self.stream.write().await.as_mut()
            .unwrap()
            .close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "".into(),
            }))
            .await
            .unwrap();
    }

    // todo request guild members, etc
}