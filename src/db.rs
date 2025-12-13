use axum::{
    extract::Query,
    http::{header::HeaderMap, Method, Uri},
};
use chrono::{DateTime, Local};
use std::str::FromStr;
use tokio::fs::{create_dir_all, read_dir, File, OpenOptions};
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

    pub async fn get_file_path(
        &self,
        full_uri: Uri,
        method: Method,
        is_yaml: bool,
        time: Option<DateTime<Local>>,
        suffix: Option<String>,
    ) -> std::path::PathBuf {
        // use now if no time given
        let time = match time {
            Some(t) => t,
            None => Local::now(),
        };

        let request_file = RequestFile {
            time: time,
            method,
            uri: full_uri.clone(),
            is_yaml,
        };

        let file_name = match suffix {
            None => request_file.to_string(),
            Some(suffix) => {
                let original = request_file.to_string();
                let new_suffix = format!("{suffix}.yaml");
                original.replace(".yaml", &new_suffix)
            }
        };

        std::path::Path::new(&self.folder).join(file_name)
    }

    pub async fn create_file(
        &self,
        full_uri: Uri,
        method: Method,
        is_yaml: bool,
        time: Option<DateTime<Local>>,
        suffix: Option<String>,
    ) -> io::Result<File> {
        // use now if no time given
        let time = match time {
            Some(t) => t,
            None => Local::now(),
        };

        let request_file = RequestFile {
            time: time,
            method,
            uri: full_uri.clone(),
            is_yaml,
        };

        let file_name = match suffix {
            None => request_file.to_string(),
            Some(suffix) => {
                let original = request_file.to_string();
                let new_suffix = format!("{suffix}.yaml");
                original.replace(".yaml", &new_suffix)
            }
        };

        let path = std::path::Path::new(&self.folder).join(file_name);
        File::create(path).await
    }

    pub async fn get_file_header(
        &self,
        headers: HeaderMap,
        params: Query<Vec<(String, String)>>,
    ) -> (String, bool) {
        let info = RequestInfo::from_parts(&headers, params.to_vec());
        match serde_yaml::to_string(&info) {
            Ok(yaml) => (yaml, true),
            Err(e) => {
                tracing::error!("Could not parse data to yaml, defaulting to debug. {e}");
                (format!("{:?}", info), false)
            }
        }
    }

    pub async fn add(
        &self,
        full_uri: Uri,
        headers: HeaderMap,
        params: Query<Vec<(String, String)>>,
        body: &str,
        method: Method,
        suffix: Option<String>,
    ) {
        let (parts_string, is_yaml) = self.get_file_header(headers, params).await;

        let now = Local::now();

        async {
            // Create the file. `File` implements `AsyncWrite`.
            let mut file = self
                .create_file(full_uri.clone(), method, is_yaml, Some(now), suffix)
                .await?;

            // Save Uri in file.
            file.write_all(format!("uri: {full_uri}\n").as_bytes())
                .await?;

            // Save request parts in file.
            file.write_all(parts_string.as_bytes()).await?;

            // Save the body, if any, into the file, indent it for YAML.
            if !body.is_empty() {
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

    pub async fn fill_file(&self, file_path: std::path::PathBuf, body: &str) {
        async {
            let mut file = OpenOptions::new()
                .append(true)
                .open(file_path)
                .await
                .unwrap();

            if !body.is_empty() {
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
