use axum::{
    extract::{Path, State},
    http::Uri,
    response::{Html, IntoResponse},
};
use minijinja::{context, Environment};
use std::cmp::Reverse;

use crate::{
    db::Db,
    request::RequestFile,
    templates::{EmbeddedTemplates, StaticFile},
};

pub async fn home(State(db): State<Db>) -> impl IntoResponse {
    let mut files = db.all().await;
    files.sort_unstable_by_key(|r| Reverse(r.time));

    let file = EmbeddedTemplates::get("index.html").unwrap();
    let html_template = String::from_utf8(file.data.to_vec()).unwrap();

    let mut env = Environment::new();
    env.add_template("index.html", &html_template).unwrap();
    let template = env.get_template("index.html").unwrap();

    let html = template
        .render(context! {
            requests => files.iter().map(|r| r.to_tuple()).collect::<Vec<(_, _, _, _, _,_)>>(),
        })
        .unwrap();

    Html(html)
}

pub async fn detail(Path(encoded_name): Path<String>) -> impl IntoResponse {
    let request_file = RequestFile::read(&encoded_name).await;
    let headers_mapping = request_file.get("headers").unwrap();
    let cookies_mapping = request_file.get("cookies").unwrap();
    let query_params_mapping = request_file.get("query_params").unwrap();

    // A body might not be present in every request
    let maybe_body = request_file
        .get("body")
        .map(|body| match body {
            // If body is valid json, pretty format it
            serde_yaml::Value::String(b) => serde_json::from_str(b)
                .map(|json: serde_json::Value| {
                    serde_json::to_string_pretty(&json).unwrap_or(b.to_string())
                })
                .unwrap_or(b.to_string()),
            _ => String::new(),
        })
        .unwrap_or_default();

    let formatted = format!(
        " \
            <pre><code class='language-yaml' id='headers-{encoded_name}'>{}</code></pre> \
            <pre><code class='language-yaml' id='cookies-{encoded_name}'>{}</code></pre> \
            <pre><code class='language-yaml' id='query-params-{encoded_name}'>{}</code></pre> \
            <pre><code class='language-yaml' id='body-{encoded_name}'>{}</code></pre> \
        ",
        serde_yaml::to_string(&headers_mapping).unwrap_or_default(),
        serde_yaml::to_string(&cookies_mapping).unwrap_or_default(),
        serde_yaml::to_string(&query_params_mapping).unwrap_or_default(),
        maybe_body,
    );

    Html(formatted)
}

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.starts_with("static/") {
        path = path.replace("static/", "");
    }

    StaticFile::get(path)
}
