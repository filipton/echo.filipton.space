use anyhow::Result;
use handler::handler;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Client, Request, Response};
use hyper_tls::HttpsConnector;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use structs::{SharedState, State};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use utils::read_static_files;

mod echo;
mod github;
mod handler;
mod login;
mod structs;
mod utils;
mod webhook;

lazy_static::lazy_static! {
    static ref IS_DEV: bool = std::env::var("DEV").is_ok();
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on {}", addr);
    if *IS_DEV {
        println!("Running in dev mode!");
    }

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let pool = PgPoolOptions::new()
        .max_connections(15)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let state = Arc::new(RwLock::new(State {
        files: read_static_files("/")?,
        clients: HashMap::new(),

        http_client: client,
        db_pool: pool,

        github_client_id: std::env::var("GITHUB_CLIENT_ID")?,
        github_client_secret: std::env::var("GITHUB_CLIENT_SECRET")?,
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
    req: Request<Body>,
    _client_addr: SocketAddr,
    state: SharedState,
) -> Result<Response<Body>> {
    let res = handler(req, _client_addr, state).await;

    if let Err(err) = &res {
        println!("Error: {:?}", err);

        if *IS_DEV {
            return Ok(Response::builder()
                .status(500)
                .body(Body::from(format!("Error: {:?}", err)))?);
        } else {
            return Ok(Response::builder()
                .status(500)
                .body(Body::from("Internal Server Error"))?);
        }
    }

    res
}
