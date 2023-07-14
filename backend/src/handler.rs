use crate::{
    echo, github, login,
    structs::{SharedState, WsMessage},
    utils::request_to_raw_http,
    webhook,
};
use anyhow::Result;
use hyper::{Body, Request, Response};
use std::net::SocketAddr;

pub async fn handler(
    mut req: Request<Body>,
    _client_addr: SocketAddr,
    state: SharedState,
) -> Result<Response<Body>> {
    let uri = req.uri().path();

    if uri == "/" && fastwebsockets::upgrade::is_upgrade_request(&req) {
        let (resp, fut) = fastwebsockets::upgrade::upgrade(&mut req)?;

        tokio::spawn(async move {
            let ws = fastwebsockets::FragmentCollector::new(fut.await.unwrap());
            echo::handle_ws(ws).await.unwrap();
        });

        return Ok(resp);
    } else if uri == "/ws" && fastwebsockets::upgrade::is_upgrade_request(&req) {
        let (resp, fut) = fastwebsockets::upgrade::upgrade(&mut req)?;
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
    } else if uri == "/oauth/login" {
        return Ok(Response::builder()
            .status(307)
            .header(
                "Location",
                format!(
                    "https://github.com/login/oauth/authorize?client_id={}&scope=user:email",
                    state.read().await.github_client_id
                ),
            )
            .body("Redirecting...".into())?);
    } else if uri == "/oauth/callback" {
        let github_code = req
            .uri()
            .query()
            .expect("Query should exist")
            .split("=")
            .nth(1)
            .expect("Code should exist");

        return login::login_github_user(
            &state,
            github::authorize_github_user(&state, github_code).await?,
        )
        .await;
    } else if uri == "/oauth/logout" {
        return login::logout_user(&state, req.headers().get("cookie")).await;
    } else {
        let state = state.read().await;
        let mut is_html = false;

        let mut file = state.files.get(uri);
        if file.is_none() {
            file = state.files.get("/index.html");

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
