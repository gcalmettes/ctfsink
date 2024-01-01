use axum::http::Method;
use chrono::Local;
use tokio::fs::{create_dir_all, File};
use tokio::io;
use tokio::io::AsyncWriteExt; // for write_all()

use crate::{config, sink::models::RequestFile};

pub async fn write_request_to_file(
    path: Option<String>,
    parts: &str,
    body: &str,
    method: Method,
    is_yaml: bool,
) {
    let settings = &config::SETTINGS;

    let now = Local::now();

    let request_file = RequestFile {
        time: now,
        method,
        path,
        is_yaml,
    };

    async {
        // Create the file. `File` implements `AsyncWrite`.
        create_dir_all(&settings.requests_folder).await?;
        let path = std::path::Path::new(&settings.requests_folder).join(request_file.to_string());
        let mut file = File::create(path).await?;

        // Copy the request parts into the file.
        file.write_all(parts.as_bytes()).await?;

        // Copy the body, if any, into the file, indent it for YAML.
        if body.len() > 0 {
            // multiline yaml string
            file.write_all("body: |\n  ".as_bytes()).await?;
            // indent each line so it because a multiline string in the yaml
            file.write_all(body.replace("\n", "\n  ").as_bytes())
                .await?;
        }

        Ok::<_, io::Error>(())
    }
    .await
    .unwrap();
}

// to prevent directory traversal attacks we ensure the path consists of exactly one normal
// component
pub fn path_is_valid(path: &str) -> bool {
    let path = std::path::Path::new(path);
    let mut components = path.components().peekable();

    if let Some(first) = components.peek() {
        if !matches!(first, std::path::Component::Normal(_)) {
            return false;
        }
    }

    components.count() == 1
}
