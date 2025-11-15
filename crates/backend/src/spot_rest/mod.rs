use crate::AppState;
use crate::middleware::clerk::ClerkState;
use crate::middleware::clerk::validate_clerk_session;
use crate::secret_str::SecretStr;
use ap_actor::order_management::OrderManagement;
use axum::Router;
use axum::routing::{get, post};
use std::time::Duration;

mod __dummy {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    pub async fn f() -> impl IntoResponse {
        StatusCode::NOT_IMPLEMENTED
    }
}

pub mod add_export;
pub mod add_order;
pub mod add_order_batch;
pub mod amend_order;
pub mod asset_pairs;
pub mod assets;
pub mod balance;
pub mod balance_ex;
pub mod cancel_all;
pub mod cancel_all_after;
pub mod cancel_order;
pub mod cancel_order_batch;
pub mod closed_orders;
pub mod credit_lines;
pub mod depth;
pub mod edit_order;
pub mod export_status;
pub mod get_websockets_token;
pub mod ledgers;
pub mod ohlc;
pub mod open_orders;
pub mod open_positions;
pub mod order_amends;
pub mod query_ledgers;
pub mod query_orders;
pub mod query_trades;
pub mod remove_export;
pub mod retrieve_export;
pub mod spread;
pub mod system_status;
pub mod ticker;
pub mod time;
pub mod trade_balance;
pub mod trade_volume;
pub mod trades;
pub mod trades_history;

pub fn routes_market_data(state: AppState) -> Router {
    Router::new()
        .route("/Time", post(time::f))
        .route("/SystemStatus", post(system_status::f))
        .route("/Assets", post(assets::f).with_state(state.clone()))
        .route(
            "/AssetPairs",
            get(asset_pairs::f)
                .post(asset_pairs::f)
                .with_state(state.clone()),
        )
        .route("/Ticker", post(ticker::f).with_state(state.clone()))
        .route("/OHLC", post(ohlc::f).with_state(state.clone()))
        .route("/Depth", post(depth::f).with_state(state.clone()))
        .route("/Trades", post(trades::f).with_state(state.clone()))
        .route("/Spread", post(spread::f).with_state(state))
}

fn routes_account_data() -> Router {
    Router::new()
        .route("/BalanceEx", post(balance_ex::f))
        .route("/Balance", post(balance::f))
        .route("/CreditLines", post(credit_lines::f))
        .route("/TradeBalance", post(trade_balance::f))
        .route("/OpenOrders", post(open_orders::f))
        .route("/ClosedOrders", post(closed_orders::f))
        .route("/QueryOrders", post(query_orders::f))
        .route("/OrderAmends", post(order_amends::f))
        .route("/TradesHistory", post(trades_history::f))
        .route("/QueryTrades", post(query_trades::f))
        .route("/OpenPositions", post(open_positions::f))
        .route("/Ledgers", post(ledgers::f))
        .route("/QueryLedgers", post(query_ledgers::f))
        .route("/TradeVolume", post(trade_volume::f))
        .route("/AddExport", post(add_export::f))
        .route("/ExportStatus", post(export_status::f))
        .route("/RetrieveExport", post(retrieve_export::f))
        .route("/RemoveExport", post(remove_export::f))
}

fn routes_trading(state: AppState) -> Router {
    let clerk_state = ClerkState::configured(
        SecretStr(
            std::env::var("CLERK_SECRET_KEY").expect("clerk bearer access token must be set"),
        ),
        std::env::var("CLERK_ISSUER")
            .unwrap_or_else(|_| "https://clerk.your-domain.com".to_owned()),
        std::env::var("CLERK_AUDIENCE").unwrap_or_else(|_| "your-app-id".to_owned()),
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

    Router::new()
        .route("/AddOrder", post(add_order::f))
        .route("/AmendOrder", post(amend_order::f))
        .route("/CancelOrder", post(cancel_order::f))
        .route("/CancelAll", post(cancel_all::f))
        .route("/CancelAllOrdersAfter", post(cancel_all_after::f))
        .route("/GetWebSocketsToken", post(get_websockets_token::f))
        .route("/AddOrderBatch", post(add_order_batch::f))
        .route("/CancelOrderBatch", post(cancel_order_batch::f))
        .route("/EditOrder", post(edit_order::f))
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(
            clerk_state.clone(),
            validate_clerk_session,
        ))
}

fn routes_funding() -> Router {
    Router::new()
    // .route("DepositMethods", post(deposit_methods::f))
    // .route("DepositAddresses", post(deposit_addresses::f))
    // .route("DepositStatus", post(deposit_status::f))
    // .route("WithdrawMethods", post(withdraw_methods::f))
    // .route("WithdrawAddresses", post(withdraw_addresses::f))
    // .route("WithdrawInfo", post(withdraw_info::f))
    // .route("Withdraw", post(withdraw::f))
    // .route("WithdrawStatus", post(withdraw_status::f))
    // .route("WithdrawCancel", post(withdraw_cancel::f))
    // .route("WalletTransfer", post(withdraw_transfer::f))
}

fn routes_subaccounts() -> Router {
    Router::new()
    // .route("CreateSubaccount", post(create_subaccount::f))
    // .route("AccountTransfer", post(account_transfer::f))
}

pub fn routes_public(state: AppState) -> Router {
    routes_market_data(state)
}

pub fn routes_private(state: AppState) -> Router {
    routes_account_data().merge(routes_trading(state))
}
