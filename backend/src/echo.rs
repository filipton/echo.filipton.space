use fastwebsockets::{FragmentCollector, WebSocketError};
use hyper::upgrade::Upgraded;
use tokio::net::unix::SocketAddr;

async fn handle_ws(
    mut ws: FragmentCollector<Upgraded>,
    client_addr: SocketAddr,
) -> Result<(), WebSocketError> {
    loop {
        tokio::select! {
            frame = ws.read_frame() => {
                let frame = frame?;

                match frame.opcode {
                    OpCode::Close => {
                        //println!("Closing connection...");
                        break;
                    }
                    OpCode::Text => {
                        let text = String::from_utf8(frame.payload.to_vec()).unwrap();
                        state.read().await.broadcast(&client_addr, WsMessage::Text(text)).await;
                        //ws.write_frame(frame).await?;
                    }
                    OpCode::Binary => {
                        state.read().await.broadcast(&client_addr, WsMessage::Binary(frame.payload.to_vec())).await;
                        //ws.write_frame(frame).await?;
                    }
                    _ => {}
                }
            },
            frame = rx.recv() => {
                if let Some(frame) = frame {
                    ws.write_frame(frame.to_frame()).await?;
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}
