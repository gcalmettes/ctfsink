use axum::{
    extract::Path,
    http::Uri,
    response::{Html, IntoResponse},
};
use minijinja::{context, Environment};
use std::cmp::Reverse;
use std::str::FromStr;
use tokio::fs;
use tokio::fs::create_dir_all;

use crate::config;
use crate::sink::models::RequestFile;
use crate::templates::EmbeddedTemplates;
use crate::templates::StaticFile;

pub async fn home() -> impl IntoResponse {
    let settings = &config::SETTINGS;
    let requests_folder = &settings.requests_folder;

    create_dir_all(requests_folder).await.unwrap();

    let mut files = vec![];
    let mut entries = fs::read_dir(requests_folder).await.unwrap();

    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Some(filename) = entry.file_name().to_str() {
            if let Ok(request_file) = RequestFile::from_str(filename) {
                files.push(request_file);
            };
        }
    }

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
    let body_mapping = request_file.get("body").unwrap_or(&serde_yaml::Value::Null);

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
        serde_yaml::to_string(&body_mapping).unwrap_or_default(),
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
