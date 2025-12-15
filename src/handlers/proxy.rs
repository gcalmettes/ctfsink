use axum::{
    body::{Body, BodyDataStream, Bytes},
    extract::{Query, Request, State},
    http::{header, header::HeaderMap, Method, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
};

use tokio::fs::File;
use tokio::io;

use tokio::io::AsyncWriteExt;

use tokio::runtime::Handle;

use axum_macros::FromRef;
use chrono::Local;
use hyper_util::client::legacy::connect::HttpConnector;
use std::pin::{pin, Pin};
use std::task::Context;
use std::task::Poll;

use futures_util::stream::Stream;

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
    let bytes = buffer_and_save(
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

    let resp = next.run(req).await;
    let resp_headers = resp.headers().clone();

    if let Some(content_type) = resp.headers().get(header::CONTENT_TYPE) {
        // non stream response (buffered)
        if !content_type.as_bytes().starts_with(b"text/event-stream") {
            let (parts, body) = resp.into_parts();

            let bytes = buffer_and_save(
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
            let resp = Response::from_parts(parts, Body::from(bytes));
            return Ok(resp);
        }
    } else {
        // in doubt, just return the response
        return Ok(resp);
    }

    // stream response (SSE)
    let (parts, body) = resp.into_parts();

    let now = Local::now();

    let (parts_string, is_yaml) = db
        .get_file_header(resp_headers.clone(), params.clone())
        .await;

    // create file and add headers
    let file_path = db
        .get_file_path(
            original_uri.clone(),
            method.clone(),
            is_yaml,
            Some(now),
            Some(String::from("_PROXY_RESP")),
        )
        .await;

    async {
        // Create the file. `File` implements `AsyncWrite`.
        let mut file = File::create(file_path.clone()).await?;
        // Save Uri in file.
        file.write_all(format!("uri: {original_uri}\n").as_bytes())
            .await?;

        // Save request parts in file.
        file.write_all(parts_string.as_bytes()).await?;

        // prepare body section
        file.write_all("body: |\n  ".as_bytes()).await?;

        Ok::<_, io::Error>(())
    }
    .await
    .unwrap();

    let body = body.into_data_stream();

    let body = Body::from_stream(SaveStream::new(body, db.clone(), file_path));

    let resp = Response::from_parts(parts, body);

    Ok(resp)
}

async fn buffer_and_save<B>(
    db: Db,
    direction: &str,
    full_uri: Uri,
    headers: HeaderMap,
    params: Query<Vec<(String, String)>>,
    method: Method,
    suffix: Option<String>,
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
        db.add(full_uri, headers, params, body, method, suffix)
            .await;
    }

    Ok(bytes)
}

struct SaveStream {
    inner: BodyDataStream,
    db: Db,
    file_path: std::path::PathBuf,
}

impl SaveStream {
    pub fn new(body: BodyDataStream, db: Db, file_path: std::path::PathBuf) -> Self {
        Self {
            inner: body,
            db: db,
            file_path: file_path,
        }
    }
}

impl Stream for SaveStream {
    type Item = Result<Bytes, axum::Error>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match pin!(&mut self.inner).as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                let body = std::str::from_utf8(&chunk).unwrap();

                let handle = Handle::current();
                let _ = handle.enter();
                futures::executor::block_on(async {
                    self.db.fill_file(self.file_path.clone(), body).await;
                });

                Poll::Ready(Some(Ok(chunk)))
            }
            x => x,
        }
    }
}
