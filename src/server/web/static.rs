//! Static files embedded in the binary.

use axum::{
    http::{header, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$OUT_DIR/static"]
pub struct StaticFiles;

pub struct StaticFile<T>(T);

impl<T> IntoResponse for StaticFile<T>
where
    T: AsRef<str>,
{
    fn into_response(self) -> axum::response::Response {
        let path = self.0.as_ref();
        match StaticFiles::get(path) {
            Some(file) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
            }
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').to_string();
    StaticFile(path)
}
