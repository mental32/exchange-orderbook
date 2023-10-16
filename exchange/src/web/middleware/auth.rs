use axum::extract::State;
use axum::http::{status, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use redis::AsyncCommands;

use crate::web::InternalApiState;

pub async fn internal_api_authentication_check<B>(
    State(state): State<InternalApiState>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let mut conn = match state.redis.get_multiplexed_tokio_connection().await {
        Ok(conn) => conn,
        Err(err) => {
            tracing::error!(?err, "failed to get redis connection");
            return status::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let map: std::collections::HashMap<String, ()> = conn.hgetall(b"").await.unwrap();

    next.run(request).await
}
