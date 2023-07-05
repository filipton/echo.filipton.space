use anyhow::Result;
use fastwebsockets::Frame;
use hyper::{Body, Request};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::UnboundedSender, RwLock};

pub type Tx = UnboundedSender<WsMessage>;
pub type SharedState = Arc<RwLock<State>>;

pub struct State {
    pub clients: HashMap<u64, Tx>,
}

impl State {
    pub async fn send(&self, to: &u64, msg: WsMessage) {
        if let Some(tx) = self.clients.get(&to) {
            tx.send(msg).unwrap();
        }
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

pub async fn request_to_raw_http(req: Request<Body>) -> Result<String> {
    let mut raw = format!(
        "{} {} {:?}\r\n",
        req.method(),
        req.uri().path(),
        req.version()
    );

    for (name, value) in req.headers() {
        raw.push_str(&format!("{}: {}\r\n", name, value.to_str()?));
    }

    let body = hyper::body::to_bytes(req.into_body()).await;
    raw.push_str(&format!("\r\n{}", String::from_utf8(body?.to_vec())?));

    Ok(raw)
}
