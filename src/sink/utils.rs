use axum::http::Method;
use chrono::Local;
use tokio::fs::{create_dir_all, File};
use tokio::io;
use tokio::io::AsyncWriteExt; // for write_all()

use crate::config;

pub async fn write_request_to_file(parts: &str, body: &str, method: Method, yaml: bool) {
    let settings = &config::SETTINGS;

    let time = Local::now().format("%Y%m%d-%H:%M:%S");
    let path = format!("{time}-{method:?}.{}", if yaml { "yaml" } else { "in" });
    async {
        // Create the file. `File` implements `AsyncWrite`.
        create_dir_all(&settings.requests_folder).await?;
        let path = std::path::Path::new(&settings.requests_folder).join(path);
        let mut file = File::create(path).await?;

        // Copy the request parts into the file.
        file.write_all(parts.as_bytes()).await?;

        // Copy the body, if any, into the file, indent it for YAML.
        if body.len() > 0 {
            file.write_all("body: |\n  ".as_bytes()).await?;
            file.write_all(body.replace("\n", "\n  ").as_bytes())
                .await?;
        }

        Ok::<_, io::Error>(())
    }
    .await
    .unwrap();
}
