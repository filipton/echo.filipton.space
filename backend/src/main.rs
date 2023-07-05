use anyhow::Result;
use fastwebsockets::upgrade::{is_upgrade_request, upgrade};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request, Response};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use structs::{SharedState, State, WsMessage};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use utils::{read_static_files, request_to_raw_http};

mod echo;
mod structs;
mod utils;
mod webhook;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on {}", addr);

    let state = Arc::new(RwLock::new(State {
        files: read_static_files("/")?,
        clients: HashMap::new(),
    }));
    println!("Loaded {} static files!", state.read().await.files.len());

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
                webhook::handle_ws(ws, &client_id, &state).await.unwrap();

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
