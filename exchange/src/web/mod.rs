//! Webserver API for the exchange

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use axum::ServiceExt;

use tower::ServiceBuilder;

use tower_http::normalize_path::NormalizePathLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

mod middleware;

mod trade_add_order;
pub use trade_add_order::TradeAddOrder;
mod trade_cancel_order;
mod trade_edit_order;

mod user_add;
mod user_delete;
mod user_edit;
mod user_get;

mod session_create;
mod session_delete;

mod public_time;

/// Error returned by the webserver.
pub type Error = axum::Error;

fn internal_server_error(message: &str) -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        message.to_owned(),
    )
        .into_response()
}

/// The state of the internal API.
#[derive(Debug, Clone)]
pub struct InternalApiState {
    pub(crate) app_cx: crate::app_cx::AppCx,
    pub(crate) redis: redis::Client,
    pub(crate) assets: Arc<HashMap<crate::asset::AssetKey, crate::Asset>>,
}

/// Router for the /trade path
///
/// This router will have the following routes:
/// - `POST /trade/order` - [`trade_add_order`]
/// - `DELETE /trade/order` - [`trade_cancel_order`]
/// - `PUT /trade/order` - [`trade_edit_order`]
///
#[track_caller]
pub fn trade_routes(state: InternalApiState) -> Router {
    let trade_order = post(trade_add_order::trade_add_order)
        .delete(trade_cancel_order::trade_cancel_order)
        .put(trade_edit_order::trade_edit_order);

    Router::new()
        .route("/trade/:asset/order", trade_order)
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::validate_session_token_redis,
        ))
        .with_state(state)
}

/// Router for the /user path
///
/// This router will have the following routes:
/// - `POST /user` - [`user_add`]
/// - `DELETE /user` - [`user_delete`]
/// - `GET /user` - [`user_get`]
/// - `PUT /user` - [`user_edit`]
///
#[track_caller]
pub fn user_routes(state: InternalApiState) -> Router {
    let user = post(user_add::user_add)
        .delete(user_delete::user_delete)
        .get(user_get::user_get)
        .put(user_edit::user_edit);

    Router::new().route("/user", user).with_state(state)
}

/// Router for the /session path
///
/// This router will have the following routes:
/// - `POST /session` - [`session_create`]
/// - `DELETE /session` - [`session_delete`]
///
#[track_caller]
pub fn session_routes(state: InternalApiState) -> Router {
    let session = post(session_create::session_create).delete(session_delete::session_delete);

    Router::new().route("/session", session).with_state(state)
}

/// Router for the /public path
pub fn public_routes() -> Router {
    Router::new().route("/public/time", axum::routing::get(public_time::public_time))
}

/// Using [`axum`], serve the internal API on the given address with the provided exchange implementation.
pub fn serve(
    address: SocketAddr,
    state: InternalApiState,
) -> impl Future<Output = Result<(), Error>> {
    let router = trade_routes(state.clone())
        .merge(user_routes(state.clone()))
        .merge(session_routes(state.clone()))
        .merge(public_routes());

    let router = Router::new().nest("/api", router);

    let x_request_id = axum::http::HeaderName::from_static("x-request-id");

    let set_request_id_layer =
        SetRequestIdLayer::new(x_request_id.clone(), MakeRequestUuid::default());

    let app = ServiceBuilder::new()
        .layer(set_request_id_layer)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(PropagateRequestIdLayer::new(x_request_id))
        .service(router);

    let app = axum::Server::bind(&address).serve(app.into_make_service());

    async move {
        tracing::info!(?address, "Serving webserver API");
        let rval = app.await.map_err(Error::new);
        tracing::warn!(?address, "Stopping webserver!");
        rval
    }
}
