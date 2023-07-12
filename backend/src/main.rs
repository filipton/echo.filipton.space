use anyhow::Result;
use fastwebsockets::upgrade::{is_upgrade_request, upgrade};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Client, Request, Response};
use hyper_tls::HttpsConnector;
use login::{login_github_user, logout_user};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use structs::{SharedState, State, WsMessage};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use utils::{read_static_files, request_to_raw_http};

use crate::github::authorize_github_user;

mod echo;
mod github;
mod login;
mod structs;
mod utils;
mod webhook;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on {}", addr);

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
    mut req: Request<Body>,
    _client_addr: SocketAddr,
    state: SharedState,
) -> Result<Response<Body>> {
    let uri = req.uri().path();

    if uri == "/" && is_upgrade_request(&req) {
        let (resp, fut) = upgrade(&mut req)?;

        tokio::spawn(async move {
            let ws = fastwebsockets::FragmentCollector::new(fut.await.unwrap());
            echo::handle_ws(ws).await.unwrap();
        });

        return Ok(resp);
    } else if uri == "/ws" && is_upgrade_request(&req) {
        let (resp, fut) = upgrade(&mut req)?;
        let client_id = if let Some(query) = req.uri().query() {
            query.parse()?
        } else {
            rand::random()
        };

        tokio::spawn(async move {
            let ws = fastwebsockets::FragmentCollector::new(fut.await.unwrap());
            webhook::handle_ws(ws, &client_id, &state).await.unwrap();

            {
                let mut state = state.write().await;
                state.clients.remove(&client_id);
            }
        });

        return Ok(resp);
    } else if uri.starts_with("/r") {
        let client_id = uri[2..].parse()?;
        let req_str = request_to_raw_http(req).await?;

        let state = state.read().await;
        _ = state
            .send(&client_id, WsMessage::Text(req_str.into()))
            .await;

        return Ok(Response::builder().status(200).body("OK".into())?);
    } else if uri == "/oauth/callback" {
        let github_code = req
            .uri()
            .query()
            .expect("Query should exist")
            .split("=")
            .nth(1)
            .expect("Code should exist");

        let res =
            login_github_user(&state, authorize_github_user(&state, github_code).await?).await;

        if let Ok(res) = res {
            return Ok(res);
        } else {
            return Ok(Response::builder().status(500).body("Internal server error".into())?);
        }
    } else if uri == "/logout" {
        let res = logout_user(&state, req.headers().get("cookie")).await;

        if let Ok(res) = res {
            return Ok(res);
        } else {
            return Ok(Response::builder()
                .status(500)
                .body("Internal server error".into())?);
        }
    } else {
        // serve static files
        let state = state.read().await;
        let mut is_html = false;

        let mut file = state.files.get(uri);
        if file.is_none() {
            // default fallback for sveltekit static files
            file = state.files.get("/index.html");

            /*
            // default file
            file = state
                .files
                .get(&format!("{}/index.html", uri.trim_end_matches('/')));
            */

            if file.is_some() {
                is_html = true;
            }
        }

        if let Some(file) = file {
            let content_type = if is_html {
                "text/html".to_string()
            } else {
                mime_guess::from_path(uri).first_or_text_plain().to_string()
            };

            return Ok(Response::builder()
                .status(200)
                .header("Content-Type", content_type)
                .body(file.to_owned().into())?);
        }
    }

    let resp = Response::builder().status(404).body("Not found".into())?;
    return Ok(resp);
}
