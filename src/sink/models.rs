use axum::http::{header::HeaderMap, Method};
use axum_extra::extract::cookie::CookieJar;
use base64::{
    alphabet,
    engine::{self, general_purpose},
    Engine as _,
};
use chrono::{offset::LocalResult, DateTime, Local, NaiveDateTime, TimeZone};
use serde::Serialize;
use std::{collections::HashMap, fmt, str::FromStr};

static TIME_FORMAT: &str = "%Y%m%d-%H:%M:%S";

const CUSTOM_ENGINE: engine::GeneralPurpose =
    engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);

#[derive(Debug)]
pub struct RequestFile {
    pub time: DateTime<Local>,
    pub method: Method,
    pub is_yaml: bool,
}

impl RequestFile {
    pub fn to_tuple(&self) -> (String, String, String, String) {
        let color = match self.method {
            Method::GET => "bg-primary",
            Method::POST => "bg-danger",
            Method::PATCH => "bg-success",
            Method::PUT => "bg-info",
            Method::OPTIONS => "bg-warning",
            _ => "bg-secondary",
        };
        (
            self.time.to_string(),
            self.method.to_string(),
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
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseRequestFileError;

impl fmt::Display for RequestFile {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let time = self.time.format(TIME_FORMAT);
        let path = format!(
            "{time}-{:?}.{}",
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
        let (method, ext) = rest.split_once(".").ok_or(ParseRequestFileError)?;

        let method = Method::from_str(method).map_err(|_| ParseRequestFileError)?;
        let is_yaml = ext == "yaml";

        let naive_fromstr =
            NaiveDateTime::parse_from_str(&format!("{}-{}", date, time), TIME_FORMAT)
                .map_err(|_| ParseRequestFileError)?;
        // let time_fromstr: DateTime<Local> = Local.from_local_datetime(&naive_fromstr).unwrap();
        if let LocalResult::Single(time) = Local.from_local_datetime(&naive_fromstr) {
            Ok(RequestFile {
                time,
                method,
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
    ) -> RequestInfo {
        let cookie_jar = CookieJar::from_headers(headers);

        let cookies = cookie_jar
            .iter()
            .map(|c| (c.name().into(), c.value_trimmed().into()))
            // .filter_map(|c| Some(c.name_value_trimmed()))
            .collect::<HashMap<String, String>>();

        let headers = headers
            .iter()
            .filter_map(|(name, value)| {
                (name != "cookie").then(|| (name.to_string(), value.to_str().unwrap_or_default()))
            })
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
