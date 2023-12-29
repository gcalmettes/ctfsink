use axum::http::header::HeaderMap;
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;
use std::collections::HashMap;

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
