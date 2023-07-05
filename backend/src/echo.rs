use fastwebsockets::{FragmentCollector, WebSocketError};
use hyper::upgrade::Upgraded;

pub async fn handle_ws(mut ws: FragmentCollector<Upgraded>) -> Result<(), WebSocketError> {
    while let Ok(frame) = ws.read_frame().await {
        ws.write_frame(frame).await?;
    }

    Ok(())
}
