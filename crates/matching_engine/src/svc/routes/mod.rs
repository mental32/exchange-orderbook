use axum::Router;
use axum::routing::post;
use common_core::web::middleware::clerk::ClerkState;
use common_core::web::middleware::clerk::validate_clerk_session;

pub mod trade_add_order;
pub mod trade_cancel_order;

pub fn routes(state: super::engine::MatchingEngineFacade) -> Router {
    let clerk_state = ClerkState::new();
    let trade_order = post(trade_add_order::handler)
        .delete(trade_cancel_order::handler)
        .layer(axum::middleware::from_fn_with_state(
            clerk_state,
            validate_clerk_session,
        ));

    Router::new()
        .route("/trade/:asset/order", trade_order)
        .with_state(state)
}
