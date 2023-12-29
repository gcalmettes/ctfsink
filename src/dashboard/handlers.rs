use axum::response::{Html, IntoResponse};
use itertools::Itertools;
use std::str::FromStr;
use tokio::fs;

use crate::config;
use crate::sink::models::RequestFile;

pub async fn home() -> impl IntoResponse {
    let settings = &config::SETTINGS;
    let requests_folder = &settings.requests_folder;

    let mut files = vec![];
    let mut entries = fs::read_dir(requests_folder).await.unwrap();

    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Some(filename) = entry.file_name().to_str() {
            if let Ok(request_file) = RequestFile::from_str(filename) {
                files.push(request_file);
            };
        }
    }

    println!("{:#?}", files.iter().sorted_by_key(|f| f.time));
    Html("Hello, World from Dashboard!")
}
