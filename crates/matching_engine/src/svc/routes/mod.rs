use super::order_management::OrderManagement;
use axum::Router;
use axum::routing::any;
use axum::routing::post;
use common_core::secret_str::SecretStr;
use common_core::web::middleware::clerk::ClerkState;
use common_core::web::middleware::clerk::validate_clerk_session;
use std::time::Duration;

pub mod trade_add_order;
pub mod trade_cancel_order;
pub mod trade_ws;

pub fn routes(state: OrderManagement) -> Router {
    let clerk_state = ClerkState::configured(
        SecretStr(
            std::env::var("CLERK_SECRET_KEY").expect("clerk bearer access token must be set"),
        ),
        std::env::var("CLERK_ISSUER")
            .unwrap_or_else(|_| "https://clerk.your-domain.com".to_string()),
        std::env::var("CLERK_AUDIENCE").unwrap_or_else(|_| "your-app-id".to_string()),
        std::env::var("CLERK_REQUIRE_AZP")
            .map(|s| s == "true" || s == "1")
            .unwrap_or(false),
        std::env::var("CLERK_JWT_LEEWAY_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60),
        std::env::var("CLERK_JWKS_CACHE_TTL")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(300)),
    );

    let trade_asset_order = post(trade_add_order::f)
        .delete(trade_cancel_order::f)
        .layer(axum::middleware::from_fn_with_state(
            clerk_state.clone(),
            validate_clerk_session,
        ));

    let trade_ws = any(trade_ws::f).layer(axum::middleware::from_fn_with_state(
        clerk_state.clone(),
        validate_clerk_session,
    ));

    Router::new()
        .route("/trade/order", trade_asset_order)
        .route("/trade/ws", trade_ws)
        .with_state(state)
}
