use crate::structs::SharedState;
use anyhow::Result;
use hyper::Request;
use serde_json::Value;

const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

pub async fn authorize_github_user(state: &SharedState, code: &str) -> Result<String> {
    let state = state.read().await;
    let url = format!(
        "{}?client_id={}&client_secret={}&code={}",
        ACCESS_TOKEN_URL, state.github_client_id, state.github_client_secret, code
    );

    let req = Request::builder()
        .method("POST")
        .uri(url)
        .header("Accept", "application/json")
        .body(hyper::Body::empty())?;

    let resp = state.http_client.request(req).await?;
    let json = hyper::body::to_bytes(resp.into_body()).await?;
    let json: Value = serde_json::from_slice(&json)?;
    let access_token = json["access_token"].as_str().unwrap();

    let github_login = get_github_user_login(&state.http_client, access_token).await?;
    Ok(github_login)
}

async fn get_github_user_login(
    http_client: &hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
    access_token: &str,
) -> Result<String> {
    let req = Request::builder()
        .method("GET")
        .uri("https://api.github.com/user")
        .header("Accept", "application/json")
        .header("User-Agent", "echo.filipton.space")
        .header("Authorization", format!("Bearer {}", access_token))
        .body(hyper::Body::empty())?;

    let resp = http_client.request(req).await?;
    let json = hyper::body::to_bytes(resp.into_body()).await?;
    let json: Value = serde_json::from_slice(&json)?;
    let login = json["login"].as_str().unwrap();

    Ok(login.to_string())
}
