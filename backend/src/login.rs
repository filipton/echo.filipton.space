use crate::structs::SharedState;
use anyhow::Result;
use hyper::{http::HeaderValue, Body, Response};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub login: String,
}

pub async fn get_user_info(
    state: &SharedState,
    cookies_header: Option<&HeaderValue>,
) -> Result<Option<UserInfo>> {
    if cookies_header.is_none() {
        return Ok(None);
    }

    let cookies: HashMap<String, String> = cookies_header
        .unwrap_or(&HeaderValue::from_static(""))
        .to_str()?
        .split("; ")
        .map(|cookie| {
            let mut split = cookie.split("=");
            let key = split.next().unwrap_or_default().to_string();
            let value = split.next().unwrap_or_default().to_string();

            (key, value)
        })
        .collect();

    let token = sqlx::types::Uuid::parse_str(cookies.get("token").unwrap_or(&"".to_string()))?;

    let state = state.read().await;

    let res = sqlx::query_as!(
        UserInfo,
        "SELECT users.id, users.login FROM users INNER JOIN sessions USING (id) WHERE sessions.token = $1",
        token
    )
    .fetch_optional(&state.db_pool)
    .await?;

    return Ok(res);
}

pub async fn login_github_user(
    state: &SharedState,
    github_info: (u64, String),
) -> Result<Response<Body>> {
    let (id, username) = github_info;
    let state = state.read().await;

    sqlx::query!(
        "INSERT INTO users (id, login) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET login = $2",
        id as i64,
        username
    )
    .execute(&state.db_pool)
    .await?;

    let token = sqlx::query_scalar!(
        "INSERT INTO sessions(id) VALUES ($1) RETURNING TOKEN",
        id as i64
    )
    .fetch_one(&state.db_pool)
    .await?;

    if token.is_none() {
        return Ok(Response::builder()
            .status(500)
            .body(format!("Internal server error!").into())?);
    }

    return Ok(Response::builder()
        .status(307)
        .header(
            "Set-Cookie",
            format!(
                "token={}; HttpOnly; Max-Age=2678400; Path=/",
                token.expect("Token is none")
            ),
        )
        .header("Location", "/")
        .body(token.expect("Token is none").to_string().into())?);
}

pub async fn logout_user(
    state: &SharedState,
    cookies_header: Option<&HeaderValue>,
) -> Result<Response<Body>> {
    if cookies_header.is_none() {
        return Ok(Response::builder()
            .status(307)
            .header("Location", "/")
            .body("".into())?);
    }

    let cookies: HashMap<String, String> = cookies_header
        .unwrap_or(&HeaderValue::from_static(""))
        .to_str()?
        .split("; ")
        .map(|cookie| {
            let mut split = cookie.split("=");
            let key = split.next().unwrap_or_default().to_string();
            let value = split.next().unwrap_or_default().to_string();

            (key, value)
        })
        .collect();

    let token = sqlx::types::Uuid::parse_str(cookies.get("token").unwrap_or(&"".to_string()))?;
    delete_sessions(state, &token).await?;

    return Ok(Response::builder()
        .status(307)
        .header(
            "Set-Cookie",
            format!("token=; HttpOnly; Path=/; Max-Age=-1"),
        )
        .header("Location", "/")
        .body(format!("").into())?);
}

async fn delete_sessions(state: &SharedState, token: &sqlx::types::Uuid) -> Result<()> {
    let state = state.read().await;

    let mut tries = 0;
    while tries < 10 {
        let res = sqlx::query!("DELETE FROM sessions WHERE token = $1", &token)
            .execute(&state.db_pool)
            .await;

        if let Ok(_) = res {
            return Ok(());
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        tries += 1;
    }

    return Err(anyhow::anyhow!("Failed to delete session"));
}
