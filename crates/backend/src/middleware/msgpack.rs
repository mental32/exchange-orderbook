use axum::body::Bytes;
use axum::extract::FromRequest;
use axum::extract::Request;
use axum::extract::rejection::BytesRejection;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::IntoResponse;
use axum::response::Response;
use futures::FutureExt;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Msgpack<T>(pub T);

impl<T, S> FromRequest<S> for Msgpack<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = MsgpackRejection;

    fn from_request<'life0, 'async_trait>(
        req: Request,
        state: &'life0 S,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, Self::Rejection>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        async move {
            if !msgpack_content_type(req.headers()) {
                return Err(MsgpackRejection::MissingMsgpackContentType);
            }

            let bytes = Bytes::from_request(req, state)
                .await
                .map_err(MsgpackRejection::BytesRejection)?;

            let value = rmp_serde::from_slice(&bytes)
                .map_err(|err| MsgpackRejection::DeserializationError(err.to_string()))?;

            Ok(Self(value))
        }
        .boxed()
    }
}

#[derive(Debug)]
pub enum MsgpackRejection {
    MissingMsgpackContentType,
    BytesRejection(BytesRejection),
    DeserializationError(String),
}

impl IntoResponse for MsgpackRejection {
    fn into_response(self) -> Response {
        match self {
            Self::MissingMsgpackContentType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected request with `Content-Type: application/msgpack`",
            )
                .into_response(),
            Self::BytesRejection(rejection) => rejection.into_response(),
            Self::DeserializationError(msg) => (
                StatusCode::BAD_REQUEST,
                format!("Failed to deserialize MessagePack: {}", msg),
            )
                .into_response(),
        }
    }
}

fn msgpack_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|ct| ct.to_str().ok())
        .is_some_and(|ct| {
            ct.starts_with("application/msgpack") || ct.starts_with("application/x-msgpack")
        })
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
            Ok(bytes) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/msgpack"),
                )],
                bytes,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}

impl<T> std::ops::Deref for Msgpack<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Msgpack<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestPayload {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_valid_msgpack_extraction() {
        let payload = TestPayload {
            name: "test".to_owned(),
            value: 42,
        };
        let msgpack_bytes = rmp_serde::to_vec(&payload).unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header(header::CONTENT_TYPE, "application/msgpack")
            .body(Body::from(msgpack_bytes))
            .unwrap();

        let result = Msgpack::<TestPayload>::from_request(req, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, payload);
    }

    #[tokio::test]
    async fn test_alternative_content_type() {
        let payload = TestPayload {
            name: "alt".to_owned(),
            value: 99,
        };
        let msgpack_bytes = rmp_serde::to_vec(&payload).unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header(header::CONTENT_TYPE, "application/x-msgpack")
            .body(Body::from(msgpack_bytes))
            .unwrap();

        let result = Msgpack::<TestPayload>::from_request(req, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, payload);
    }

    #[tokio::test]
    async fn test_missing_content_type() {
        let payload = TestPayload {
            name: "test".to_owned(),
            value: 42,
        };
        let msgpack_bytes = rmp_serde::to_vec(&payload).unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .body(Body::from(msgpack_bytes))
            .unwrap();

        let result = Msgpack::<TestPayload>::from_request(req, &()).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            MsgpackRejection::MissingMsgpackContentType => {}
            other => panic!("Expected MissingMsgpackContentType, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_wrong_content_type() {
        let payload = TestPayload {
            name: "test".to_owned(),
            value: 42,
        };
        let msgpack_bytes = rmp_serde::to_vec(&payload).unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(msgpack_bytes))
            .unwrap();

        let result = Msgpack::<TestPayload>::from_request(req, &()).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            MsgpackRejection::MissingMsgpackContentType => {}
            other => panic!("Expected MissingMsgpackContentType, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_invalid_msgpack_data() {
        let invalid_bytes = vec![0xFF, 0xFF, 0xFF, 0xFF];

        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header(header::CONTENT_TYPE, "application/msgpack")
            .body(Body::from(invalid_bytes))
            .unwrap();

        let result = Msgpack::<TestPayload>::from_request(req, &()).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            MsgpackRejection::DeserializationError(_) => {}
            other => panic!("Expected DeserializationError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_response_serialization() {
        let payload = TestPayload {
            name: "response".to_owned(),
            value: 123,
        };
        let msgpack = Msgpack(payload.clone());

        let response = msgpack.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/msgpack"
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let deserialized: TestPayload = rmp_serde::from_slice(&body_bytes).unwrap();
        assert_eq!(deserialized, payload);
    }

    #[tokio::test]
    async fn test_rejection_status_codes() {
        let missing_ct = MsgpackRejection::MissingMsgpackContentType;
        let response = missing_ct.into_response();
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

        let deser_err = MsgpackRejection::DeserializationError("test error".to_owned());
        let response = deser_err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
