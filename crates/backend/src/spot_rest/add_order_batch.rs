use crate::middleware::clerk::Clerk;
use crate::spot_rest::add_order::TradeAddOrder;
use ap_actor::proc::OrderDescription;
use ap_actor::proc::OrderResult;
use ap_actor::proc_router::ProcRouter;
use axum::Extension;
use axum::extract::Json;
use axum::extract::State;
use axum::http::StatusCode;
use sqlx::PgPool;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AddOrderBatch {
    pub nonce: u64,
    pub orders: Vec<TradeAddOrder>,
    pub pair: String,
    pub asset_class: Option<String>,
    pub deadline: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub validate: Option<bool>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AddOrderBatchResponse {
    pub orders: Vec<OrderResult>,
}

pub async fn f(
    State(proc_router): State<ProcRouter>,
    State(users): State<crate::Users>,
    State(pg_pool): State<PgPool>,
    Extension(clerk): Extension<Clerk>,
    Json(AddOrderBatch {
        nonce: _,
        mut orders,
        pair,
        asset_class: _,
        deadline: _,
        validate: _,
    }): Json<AddOrderBatch>,
) -> Result<Json<AddOrderBatchResponse>, (StatusCode, &'static str)> {
    // Validate batch size (2-15 orders per Kraken spec)
    if orders.len() < 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            "batch must contain at least 2 orders",
        ));
    }
    if orders.len() > 15 {
        return Err((StatusCode::BAD_REQUEST, "batch cannot exceed 15 orders"));
    }

    // Verify the pair is enabled and get base_quote
    let Some(base_quote) = proc_router.is_pair_enabled(&pair) else {
        tracing::warn!(pair = %pair, "asset pair not enabled");
        return Err((StatusCode::NOT_FOUND, "asset pair not enabled"));
    };

    // Verify all orders are for the specified pair
    for order in &mut orders {
        if order.symbol != pair {
            return Err((
                StatusCode::BAD_REQUEST,
                "all orders in batch must be for the same pair as specified in pair field",
            ));
        }
    }

    let user_id = users.to_user_pk(clerk.user_id(), pg_pool).await;
    let order_tickets: Vec<_> = orders.into_iter().map(|o| o.into()).collect();

    match proc_router
        .place_order_batch(base_quote, user_id, order_tickets)
        .await
    {
        Ok(results) => Ok(Json(AddOrderBatchResponse { orders: results })),
        Err(err) => {
            tracing::error!(?err, "failed to place order batch");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to place order batch",
            ))
        }
    }
}
