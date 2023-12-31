use anyhow::Result;
use fastwebsockets::Frame;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::UnboundedSender, RwLock};

pub type Tx = UnboundedSender<WsMessage>;
pub type SharedState = Arc<RwLock<State>>;

pub struct State {
    pub files: HashMap<String, Vec<u8>>,
    pub clients: HashMap<u64, Tx>,

    pub http_client: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
    pub db_pool: sqlx::PgPool,

    pub github_client_id: String,
    pub github_client_secret: String,
}

impl State {
    pub async fn send(&self, to: &u64, msg: WsMessage) -> Result<()> {
        if let Some(tx) = self.clients.get(&to) {
            tx.send(msg)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),

    /// Send a pong message with the given data.
    Pong(Vec<u8>),

    /// Close the connection with the given code and reason.
    ///
    /// u16 is the status code
    /// String is the reason
    Close(u16, String),
}

impl WsMessage {
    pub fn to_frame(&self) -> Frame {
        match self {
            WsMessage::Text(text) => Frame::text(text.as_bytes().into()),
            WsMessage::Binary(data) => Frame::binary(data.as_slice().into()),
            WsMessage::Pong(data) => Frame::pong(data.as_slice().into()),
            WsMessage::Close(code, reason) => Frame::close(*code, reason.as_bytes()),
        }
    }
}
