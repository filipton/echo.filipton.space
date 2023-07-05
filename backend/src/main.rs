use crate::structs::WsMessage;
use anyhow::Result;
use fastwebsockets::upgrade::{is_upgrade_request, upgrade};
use fastwebsockets::{FragmentCollector, WebSocketError};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Body, Request, Response};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use structs::{SharedState, State};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

mod echo;
mod structs;

async fn handle_ws(
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

async fn request_handler(
    mut req: Request<Body>,
    _client_addr: SocketAddr,
    state: SharedState,
) -> Result<Response<Body>> {
    let uri = req.uri().path();

    if uri.starts_with("/r") {
        let client_id = uri[2..].parse()?;
        let req_str = request_to_raw_http(req).await?;

        let state = state.read().await;
        state
            .send(&client_id, WsMessage::Text(req_str.into()))
            .await;

        return Ok(Response::builder().status(200).body("OK".into())?);
    }

    match uri {
        "/" => {
            if is_upgrade_request(&req) {
                let (resp, fut) = upgrade(&mut req)?;

                tokio::spawn(async move {
                    let ws = fastwebsockets::FragmentCollector::new(fut.await.unwrap());
                    echo::handle_ws(ws).await.unwrap();
                });

                return Ok(resp);
            }

            let resp = Response::builder()
                .status(200)
                .body("TODO: Main page".into())?;
            return Ok(resp);
        }
        "/ws" => {
            let (response, fut) = upgrade(&mut req)?;
            let client_id = rand::random();

            tokio::spawn(async move {
                let ws = fastwebsockets::FragmentCollector::new(fut.await.unwrap());
                handle_ws(ws, &client_id, &state).await.unwrap();

                {
                    let mut state = state.write().await;
                    state.clients.remove(&client_id);
                }
            });

            return Ok(response);
        }
        _ => {
            let resp = Response::builder().status(404).body("Not found".into())?;
            return Ok(resp);
        }
    }
}

async fn request_to_raw_http(req: Request<Body>) -> Result<String> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on {}", addr);

    let state = Arc::new(RwLock::new(State {
        clients: HashMap::new(),
    }));

    loop {
        let (stream, client_addr) = listener.accept().await?;
        let state = state.clone();

        tokio::task::spawn(async move {
            if let Err(err) = Http::new()
                .serve_connection(
                    stream,
                    service_fn(move |req| request_handler(req, client_addr, state.clone())),
                )
                .with_upgrades()
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
