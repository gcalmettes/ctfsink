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
            requests => files.iter().map(|r| r.to_tuple()).collect::<Vec<(_, _, _, _)>>(),
        })
        .unwrap();

    Html(html)
}

pub async fn detail(Path(encoded): Path<String>) -> impl IntoResponse {
    let request_file = RequestFile::decode_name(&encoded);
    let settings = &config::SETTINGS;
    let path = std::path::Path::new(&settings.requests_folder).join(request_file.to_string());
    let file_content = fs::read_to_string(path).await.unwrap_or("".to_string());

    let mut content = file_content.split("body: |\n");
    let info = content.next().unwrap_or("").trim();
    let body = content.last();

    let info_mapping: serde_yaml::Mapping = serde_yaml::from_str(&info).unwrap_or_default();
    let headers_mapping = info_mapping.get("headers").unwrap();
    let cookies_mapping = info_mapping.get("cookies").unwrap();
    let query_params_mapping = info_mapping.get("query_params").unwrap();

    let formatted = format!(
        " \
            <pre><code class='language-yaml' id='headers-{encoded}'>{}</code></pre> \
            <pre><code class='language-yaml' id='cookies-{encoded}'>{}</code></pre> \
            <pre><code class='language-yaml' id='query-params-{encoded}'>{}</code></pre> \
            <pre><code class='language-yaml' id='body-{encoded}'>{}</code></pre> \
        ",
        serde_yaml::to_string(&headers_mapping).unwrap(),
        serde_yaml::to_string(&cookies_mapping).unwrap(),
        serde_yaml::to_string(&query_params_mapping).unwrap(),
        body.unwrap_or_default()
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
