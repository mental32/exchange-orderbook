use std::future::Future;
use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Router,
};

use crate::Exchange;

pub mod trade_add_order;
pub mod trade_cancel_order;
pub mod trade_edit_order;

#[derive(Debug, Clone)]
pub struct InternalApiState<E> {
    pub exchange: E,
}

/// Create a [`axum::Router`] for the internal API.
///
/// This router will have the following routes:
/// - `POST /trade/order` - [`trade_add_order`]
/// - `DELETE /trade/order` - [`trade_cancel_order`]
/// - `PUT /trade/order` - [`trade_edit_order`]
/// - `GET /account` - [`unimplemented!`]
///
#[track_caller]
pub fn internal_api_router<E>(exchange: E) -> Router
where
    E: Exchange + Clone + Send + Sync + 'static,
{
    let trade_order = post(trade_add_order::trade_add_order)
        .delete(trade_cancel_order::trade_cancel_order)
        .put(trade_edit_order::trade_edit_order);

    let account_balance = get(|| async { todo!("account/balance") });

    let funding_deposit = post(|| async { todo!("funding/deposit") });

    Router::new()
        .route("/trade/order", trade_order)
        .route("/account/balance", account_balance)
        .route("/account/deposit", funding_deposit)
        .with_state(InternalApiState { exchange })
}

/// Using [`axum`], serve the internal API on the given address with the provided exchange implementation.
pub fn serve<E>(
    address: SocketAddr,
    exchange: E,
) -> impl Future<Output = Result<(), Box<dyn std::error::Error>>>
where
    E: Exchange + Clone + Send + Sync + 'static,
{
    let router = internal_api_router(exchange);

    let app = axum::Server::bind(&address).serve(router.into_make_service());

    async move {
        tracing::info!("Serving webserver API");
        app.await?;
        Ok(())
    }
}
