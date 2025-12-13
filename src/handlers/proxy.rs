use axum::{
    body::{Body, Bytes},
    extract::{Query, Request, State},
    http::{header::HeaderMap, Method, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_macros::FromRef;
use hyper_util::client::legacy::connect::HttpConnector;

use http_body_util::BodyExt;

use crate::db::Db;

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

#[derive(FromRef, Clone)]
pub struct ProxyState {
    pub client: Client,
    pub url_to: String,
}

pub async fn proxy(
    State(client): State<Client>,
    State(url_to): State<String>,
    mut req: Request,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let uri = format!("{url_to}{path_query}");

    *req.uri_mut() = Uri::try_from(uri).unwrap();

    Ok(client
        .request(req)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}

pub async fn save_request_and_response(
    State(db): State<Db>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let original_uri = req.uri().clone();
    let headers = req.headers().clone();
    let method = req.method().clone();
    let params: Query<Vec<(String, String)>> = Query::try_from_uri(req.uri()).unwrap();

    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print(
        db.clone(),
        "request",
        original_uri.clone(),
        headers.clone(),
        params.clone(),
        method.clone(),
        Some(String::from("_PROXY_REQ")),
        body,
    )
    .await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;
    let resp_headers = res.headers().clone();

    let (parts, body) = res.into_parts();

    let bytes = buffer_and_print(
        db,
        "response",
        original_uri,
        resp_headers,
        params,
        method,
        Some(String::from("_PROXY_RESP")),
        body,
    )
    .await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(
    db: Db,
    direction: &str,
    full_uri: Uri,
    headers: HeaderMap,
    params: Query<Vec<(String, String)>>,
    method: Method,
    prefix: Option<String>,
    body: B,
) -> Result<Bytes, (StatusCode, String)>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {direction} body: {err}"),
            ));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        // tracing::debug!("{direction} body = {body:?}");

        db.add(full_uri, headers, params, body, method, prefix)
            .await;
    }

    Ok(bytes)
}
