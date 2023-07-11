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
    let state = state.read().await;

    Ok(Response::builder()
        .status(200)
        .body(format!("dsa").into())?)
}
