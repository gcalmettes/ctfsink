use axum::{
    extract::Query,
    http::{header::HeaderMap, Method, Uri},
};
use chrono::Local;
use std::str::FromStr;
use tokio::fs::{create_dir_all, read_dir, File};
use tokio::io;
use tokio::io::AsyncWriteExt; // for write_all()

use crate::{
    config,
    request::{RequestFile, RequestInfo},
};

#[derive(Clone)]
pub struct Db {
    // pub files: Arc<Mutex<Vec<RequestFile>>>,
    // pub files: Arc<Mutex<Vec<RequestFile>>>,
    pub folder: String,
}

impl Db {
    pub async fn new() -> Db {
        let settings = &config::SETTINGS;
        let folder = &settings.requests_folder;
        // ensure folder exists
        create_dir_all(folder).await.unwrap();
        Db {
            folder: folder.to_string(),
        }
    }

    pub async fn all(&self) -> Vec<RequestFile> {
        let mut entries = read_dir(&self.folder).await.unwrap();
        let mut files = vec![];

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(filename) = entry.file_name().to_str() {
                if let Ok(request_file) = RequestFile::from_str(filename) {
                    files.push(request_file);
                };
            }
        }
        files
    }

    pub async fn add(
        &self,
        full_uri: Uri,
        headers: HeaderMap,
        params: Query<Vec<(String, String)>>,
        body: &str,
        method: Method,
    ) {
        let info = RequestInfo::from_parts(&headers, params.to_vec());
        let (parts_string, is_yaml) = match serde_yaml::to_string(&info) {
            Ok(yaml) => (yaml, true),
            Err(e) => {
                tracing::error!("Could not parse data to yaml, defaulting to debug. {e}");
                (format!("{:?}", info), false)
            }
        };

        let now = Local::now();

        let request_file = RequestFile {
            time: now,
            method,
            uri: full_uri.clone(),
            is_yaml,
        };

        async {
            // Create the file. `File` implements `AsyncWrite`.
            let path = std::path::Path::new(&self.folder).join(request_file.to_string());
            let mut file = File::create(path).await?;

            // Save Uri in file.
            file.write_all(format!("uri: {full_uri}\n").as_bytes())
                .await?;

            // Save request parts in file.
            file.write_all(parts_string.as_bytes()).await?;

            // Save the body, if any, into the file, indent it for YAML.
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
}
