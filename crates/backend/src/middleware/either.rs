use axum::body::Body;
use axum::body::Bytes;
use axum::extract::FromRequest;
use axum::extract::FromRequestParts;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use futures::FutureExt;
use futures::StreamExt;
use tonic::IntoRequest;

pub struct Parts<T>(pub T);

impl<T, S> FromRequest<S> for Parts<T>
where
    T: FromRequestParts<S>,
    S: Send + Sync,
{
    type Rejection = <T as FromRequestParts<S>>::Rejection;

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
            let (mut parts, _) = req.into_parts();
            let t = <T as FromRequestParts<S>>::from_request_parts(&mut parts, state).await?;
            Ok(Parts(t))
        }
        .boxed()
    }
}

#[derive(Debug)]
pub enum Either<T, U> {
    Left(T),
    Right(U),
}

impl<T, U, S> FromRequest<S> for Either<T, U>
where
    T: FromRequest<S>,
    T::Rejection: Send,
    U: FromRequest<S>,
    U::Rejection: Send,
    S: Send + Sync,
{
    type Rejection = EitherRejection<T::Rejection, U::Rejection>;

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
            let (parts, body) = req.into_parts();
            let bytes = Bytes::from_request(Request::from_parts(parts.clone(), body), state)
                .await
                .map_err(|_| EitherRejection::FailedToReadBody)?;

            let req_t = Request::from_parts(parts.clone(), axum::body::Body::from(bytes.clone()));
            let err_t = match T::from_request(req_t, state).await {
                Ok(value) => {
                    return Ok(Either::Left(value));
                }
                Err(err) => err,
            };

            let req_u = Request::from_parts(parts, axum::body::Body::from(bytes));
            match U::from_request(req_u, state).await {
                Ok(value) => Ok(Either::Right(value)),
                Err(err_u) => Err(EitherRejection::Both(err_t, err_u)),
            }
        }
        .boxed()
    }
}

#[derive(Debug)]
pub enum EitherRejection<L, R> {
    Both(L, R),
    FailedToReadBody,
}

impl<L, R> IntoResponse for EitherRejection<L, R>
where
    L: IntoResponse,
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let this = match self {
            Self::Both(left, right) => {
                let (left, right) = (left.into_response(), right.into_response());
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
            Self::FailedToReadBody => {
                (StatusCode::BAD_REQUEST, "Failed to read request body").into_response()
            }
        };
        dbg!(&this);
        this
    }
}

#[cfg(test)]
mod tests {
    use super::super::msgpack::Msgpack;
    use super::*;
    use axum::Json;
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::header;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestPayload {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_json_extraction() {
        let payload = TestPayload {
            name: "test".to_owned(),
            value: 42,
        };
        let json_bytes = serde_json::to_vec(&payload).unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json_bytes))
            .unwrap();

        let result =
            Either::<Json<TestPayload>, Msgpack<TestPayload>>::from_request(req, &()).await;
        assert!(result.is_ok());
        let either = result.unwrap();
        match either {
            Either::Left(Json(data)) => assert_eq!(data, payload),
            Either::Right(_) => panic!("Expected Left variant"),
        }
    }

    #[tokio::test]
    async fn test_msgpack_extraction() {
        let payload = TestPayload {
            name: "msgpack_test".to_owned(),
            value: 99,
        };
        let msgpack_bytes = rmp_serde::to_vec(&payload).unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header(header::CONTENT_TYPE, "application/msgpack")
            .body(Body::from(msgpack_bytes))
            .unwrap();

        let result =
            Either::<Json<TestPayload>, Msgpack<TestPayload>>::from_request(req, &()).await;
        assert!(result.is_ok());
        let either = result.unwrap();
        match either {
            Either::Right(Msgpack(data)) => assert_eq!(data, payload),
            Either::Left(_) => panic!("Expected Right variant"),
        }
    }

    #[tokio::test]
    async fn test_both_fail() {
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from("not valid json or msgpack"))
            .unwrap();

        let result =
            Either::<Json<TestPayload>, Msgpack<TestPayload>>::from_request(req, &()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            EitherRejection::Both(_, _) => {}
            other => panic!("Expected Both rejection, got {:?}", other),
        }
    }
}
