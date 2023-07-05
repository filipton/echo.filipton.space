use fastwebsockets::{FragmentCollector, WebSocketError};
use hyper::upgrade::Upgraded;

use crate::structs::{SharedState, WsMessage};

pub async fn handle_ws(
    mut ws: FragmentCollector<Upgraded>,
    client_id: &u64,
    state: &SharedState,
) -> Result<(), WebSocketError> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    {
        _ = tx.send(WsMessage::Binary(client_id.to_be_bytes().to_vec()));

        let mut state = state.write().await;
        state.clients.insert(*client_id, tx);
    }

    while let Some(msg) = rx.recv().await {
        ws.write_frame(msg.to_frame()).await?;
    }

    Ok(())
}
