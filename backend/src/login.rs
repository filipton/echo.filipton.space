use std::collections::HashMap;

use crate::structs::SharedState;
use anyhow::Result;
use hyper::{http::HeaderValue, Request, Response};

pub async fn login_github_user<T>(
    state: &SharedState,
    github_info: (u64, String),
) -> Result<Response<T>>
where
    T: From<String>,
{
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
                token.unwrap()
            ),
        )
        .header("Location", "/")
        .body(token.unwrap().to_string().into())?);
}

pub async fn logout_user<T>(
    state: &SharedState,
    cookies_header: Option<&HeaderValue>,
) -> Result<Response<T>>
where
    T: From<String>,
{
    let cookies: HashMap<String, String> = cookies_header
        .unwrap_or(&HeaderValue::from_static(""))
        .to_str()?
        .split("; ")
        .map(|cookie| {
            let mut split = cookie.split("=");
            let key = split.next().unwrap().to_string();
            let value = split.next().unwrap().to_string();

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

    loop {
        let res = sqlx::query!("DELETE FROM sessions WHERE token = $1", &token)
            .execute(&state.db_pool)
            .await;

        if let Ok(_) = res {
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    return Ok(());
}
