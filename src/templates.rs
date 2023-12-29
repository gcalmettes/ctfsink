use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;
use std::marker::PhantomData;

#[derive(RustEmbed)]
#[folder = "src/templates"]
pub struct EmbeddedTemplates;

#[derive(RustEmbed)]
#[folder = "static"]
pub struct StaticDir;

pub type StaticFile<T> = EmbeddedFile<StaticDir, T>;

pub struct EmbeddedFile<E, T> {
    pub path: T,
    embed: PhantomData<E>,
}

impl<E, T> EmbeddedFile<E, T> {
    pub fn get(path: T) -> Self {
        Self {
            path,
            embed: PhantomData,
        }
    }
}

impl<E, T> IntoResponse for EmbeddedFile<E, T>
where
    E: RustEmbed,
    T: AsRef<str>,
{
    fn into_response(self) -> Response {
        let path: &str = self.path.as_ref();
        match E::get(path) {
            Some(content) => {
                let body = Body::from(content.data);
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .body(body)
                    .unwrap()
            }
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }
}
