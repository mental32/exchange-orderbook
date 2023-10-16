use axum::body::{Bytes, HttpBody};
use axum::extract::FromRequest;
use axum::http::{header, HeaderMap, HeaderValue, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{async_trait, BoxError};

use serde::{de::DeserializeOwned, Serialize};

/// Msgpack Extractor / Response.
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Msgpack<T>(pub T);

#[async_trait]
impl<T, S, B> FromRequest<S, B> for Msgpack<T>
where
    T: DeserializeOwned,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        if msgpack_content_type(req.headers()) {
            let bytes = Bytes::from_request(req, state)
                .await
                .map_err(|_| unimplemented!())?;

            let value = match rmp_serde::from_slice(&bytes) {
                Ok(value) => value,
                Err(_err) => {
                    // let rejection = match err.inner().classify() {
                    //     serde_json::error::Category::Data => JsonDataError::from_err(err).into(),
                    //     serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
                    //         JsonSyntaxError::from_err(err).into()
                    //     }
                    //     serde_json::error::Category::Io => {
                    //         if cfg!(debug_assertions) {
                    //             // we don't use `serde_json::from_reader` and instead always buffer
                    //             // bodies first, so we shouldn't encounter any IO errors
                    //             unreachable!()
                    //         } else {
                    //             JsonSyntaxError::from_err(err).into()
                    //         }
                    //     }
                    // };
                    // let rejection = ();
                    // return Err(unimplemented!());
                    unimplemented!()
                }
            };

            Ok(Self(value))
        } else {
            Err((StatusCode::BAD_REQUEST, "no msgpack content type"))
        }
    }
}

fn msgpack_content_type(headers: &HeaderMap) -> bool {
    let content_type = if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
        content_type
    } else {
        return false;
    };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        return false;
    };

    let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
        mime
    } else {
        return false;
    };

    let is_msgpack_content_type = mime.type_() == "application"
        && (mime.subtype() == "msgpack" || mime.suffix().map_or(false, |name| name == "msgpack"));

    is_msgpack_content_type
}

impl<T> From<T> for Msgpack<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> IntoResponse for Msgpack<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match rmp_serde::to_vec(&self.0) {
            Ok(v) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::APPLICATION_MSGPACK.as_ref()),
                )],
                v,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}
