use crate::structs::SharedState;
use anyhow::Result;
use hyper::Response;

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
        .status(200)
        .header("Set-Cookie", format!("token={}; HttpOnly", token.unwrap()))
        .body(token.unwrap().to_string().into())?);
}
