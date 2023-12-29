use axum::{
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
            requests => files.iter().map(|r| r.to_tuple()).collect::<Vec<(_, _, _)>>(),
        })
        .unwrap();

    Html(html)
}

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.starts_with("static/") {
        path = path.replace("static/", "");
    }

    StaticFile::get(path)
}

// use minijinja::{context, Environment};

//     let file = EmbeddedTemplates::get("view-feedback.html").unwrap();
//     let mut env = Environment::new();
//     let html_template = String::from_utf8(file.data.to_vec()).unwrap();
//     env.add_template("view-feedback.html", &html_template)
//         .unwrap();
//     let template = env.get_template("view-feedback.html").unwrap();

//     let html = template
//         .render(context! {
//             email => comment.email,
//             content => comment.message
//         })
//         .unwrap();
