use axum::{
    extract::{OriginalUri, Query},
    http::{header::HeaderMap, Method, StatusCode, Uri},
    response::IntoResponse,
};
use serde_yaml;

use crate::{sink::models::RequestInfo, utils};

pub async fn root(
    OriginalUri(original_uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
    params: Query<Vec<(String, String)>>,
    body: String,
) -> Result<impl IntoResponse, (StatusCode, ())> {
    save_request_to_file(original_uri, method, headers, params, body).await
}

pub async fn any_path(
    OriginalUri(original_uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
    params: Query<Vec<(String, String)>>,
    body: String,
) -> Result<impl IntoResponse, (StatusCode, ())> {
    save_request_to_file(original_uri, method, headers, params, body).await
}

pub async fn save_request_to_file(
    full_uri: Uri,
    method: Method,
    headers: HeaderMap,
    params: Query<Vec<(String, String)>>,
    body: String,
) -> Result<impl IntoResponse, (StatusCode, ())> {
    let info = RequestInfo::from_parts(&headers, params.to_vec());
    let (data_string, ok) = match serde_yaml::to_string(&info) {
        Ok(yaml) => (yaml, true),
        Err(e) => {
            tracing::error!("Could not parse data to yaml, defaulting to debug. {e}");
            (format!("{:?}", info), false)
        }
    };
    utils::write_request_to_file(full_uri, &data_string, &body, method, ok).await;
    Ok(())
}
