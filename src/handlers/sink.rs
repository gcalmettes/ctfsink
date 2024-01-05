use axum::{
    extract::{OriginalUri, Query, State},
    http::{header::HeaderMap, Method, StatusCode},
    response::IntoResponse,
};

use crate::db::Db;

pub async fn root(
    State(db): State<Db>,
    OriginalUri(original_uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
    params: Query<Vec<(String, String)>>,
    body: String,
) -> Result<impl IntoResponse, (StatusCode, ())> {
    db.add(original_uri, headers, params, &body, method).await;
    Ok(())
}

pub async fn any_path(
    State(db): State<Db>,
    OriginalUri(original_uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
    params: Query<Vec<(String, String)>>,
    body: String,
) -> Result<impl IntoResponse, (StatusCode, ())> {
    db.add(original_uri, headers, params, &body, method).await;
    Ok(())
}
