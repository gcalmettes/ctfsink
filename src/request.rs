use axum::http::{header::HeaderMap, Method, Uri};
use axum_extra::extract::cookie::CookieJar;
use base64::{
    alphabet,
    engine::{self, general_purpose},
    Engine as _,
};
use chrono::{offset::LocalResult, DateTime, Local, NaiveDateTime, TimeZone};
use ellipse::Ellipse;
use serde::Serialize;
use std::{collections::HashMap, fmt, str::FromStr};

use crate::{config, db::Db};

static TIME_FORMAT: &str = "%Y%m%d-%H:%M:%S";

const CUSTOM_ENGINE: engine::GeneralPurpose =
    engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);

// TODO: add Mutex or RWLock system to ensure files are accessed safely
// Do that probably at the db.rs level
#[derive(Debug)]
pub struct RequestFile {
    // pub mu: Mutex<()>,
    pub time: DateTime<Local>,
    pub method: Method,
    pub uri: Uri,
    pub is_yaml: bool,
}

impl RequestFile {
    pub fn to_tuple(&self) -> (String, String, String, String, String, String) {
        let color = match self.method {
            Method::GET => "bg-primary",
            Method::POST => "bg-danger",
            Method::PATCH => "bg-success",
            Method::PUT => "bg-info",
            Method::OPTIONS => "bg-warning",
            _ => "bg-secondary",
        };
        let uri = self
            .uri
            .to_string()
            .as_str()
            .truncate_ellipse(32)
            .to_string();
        (
            self.time.format("%Y/%m/%d").to_string(),
            self.time.format("%H:%M:%S").to_string(),
            self.method.to_string(),
            uri,
            color.to_string(),
            self.encoded_name(),
        )
    }

    pub fn encoded_name(&self) -> String {
        let mut buf = String::new();
        CUSTOM_ENGINE.encode_string(self.to_string(), &mut buf);
        buf
    }

    pub fn decode_name(name: &str) -> String {
        String::from_utf8(CUSTOM_ENGINE.decode(name).unwrap()).unwrap()
    }

    pub async fn read(encoded_name: &str) -> serde_yaml::Mapping {
        let request_file_name = RequestFile::decode_name(&encoded_name);

        let settings = &config::SETTINGS;

        let path =
            std::path::Path::new(&settings.requests_folder).join(request_file_name.to_string());

        if !Db::path_is_valid(&request_file_name) || !path.exists() {
            let mut fake = serde_yaml::Mapping::new();
            for section in ["headers", "cookies", "query_params", "body"] {
                fake.insert(
                    serde_yaml::Value::String(section.to_string()),
                    serde_yaml::Value::Null,
                );
            }
            return fake;
        }
        let file_content = tokio::fs::read_to_string(path)
            .await
            .unwrap_or("".to_string());

        serde_yaml::from_str(&file_content).unwrap_or_default()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseRequestFileError;

impl fmt::Display for RequestFile {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let uri = self.uri.to_string().replace("/", "|");
        let time = self.time.format(TIME_FORMAT);
        let path = format!(
            "{time}-{:?}-{uri}.{}",
            self.method,
            if self.is_yaml { "yaml" } else { "in" }
        );
        fmt.write_str(&path)?;
        Ok(())
    }
}

impl FromStr for RequestFile {
    type Err = ParseRequestFileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (date, rest) = s.split_once("-").ok_or(ParseRequestFileError)?;
        let (time, rest) = rest.split_once("-").ok_or(ParseRequestFileError)?;
        let (method, rest) = rest.split_once("-").ok_or(ParseRequestFileError)?;
        let (uri, ext) = rest.rsplit_once(".").ok_or(ParseRequestFileError)?;

        let method = Method::from_str(method).map_err(|_| ParseRequestFileError)?;
        let uri = Uri::from_str(&uri.replace("|", "/")).unwrap_or_default();
        let is_yaml = ext == "yaml";

        let naive_fromstr =
            NaiveDateTime::parse_from_str(&format!("{}-{}", date, time), TIME_FORMAT)
                .map_err(|_| ParseRequestFileError)?;
        if let LocalResult::Single(time) = Local.from_local_datetime(&naive_fromstr) {
            Ok(RequestFile {
                // mu: Mutex::new(()),
                time,
                method,
                uri,
                is_yaml,
            })
        } else {
            Err(ParseRequestFileError)
        }
    }
}

#[derive(Serialize, Debug)]
pub struct RequestInfo<'a> {
    headers: HashMap<String, &'a str>,
    cookies: HashMap<String, String>,
    query_params: HashMap<String, Vec<String>>,
}

impl RequestInfo<'_> {
    pub fn from_parts<'a>(
        headers: &'a HeaderMap,
        query_params: Vec<(String, String)>,
    ) -> RequestInfo<'a> {
        let cookie_jar = CookieJar::from_headers(headers);

        let cookies = cookie_jar
            .iter()
            .map(|c| (c.name().into(), c.value_trimmed().into()))
            .collect::<HashMap<String, String>>();

        let headers = headers
            .iter()
            .filter(|&(name, _)| (name != "cookie"))
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or_default()))
            .collect::<HashMap<String, &str>>();

        // preserve duplicate keys for params
        let query_params = query_params.into_iter().fold(
            HashMap::<String, Vec<String>>::new(),
            |mut params, (name, value)| {
                let p = params.entry(name).or_default();
                p.push(value);
                params
            },
        );

        RequestInfo {
            headers,
            cookies,
            query_params,
        }
    }
}
