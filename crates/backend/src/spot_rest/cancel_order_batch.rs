use crate::middleware::clerk::Clerk;
use ap_actor::order_management::CancelOrderBy;
use ap_actor::order_management::OrderManagement;
use axum::Extension;
use axum::extract::Json;
use axum::extract::State;
use axum::http::StatusCode;
use matching_engine::order_uuid::OrderUuid;
use sqlx::PgPool;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum TxIdOrUserref {
    TxId(uuid::Uuid),
    Userref(u32),
}

impl From<TxIdOrUserref> for CancelOrderBy {
    fn from(value: TxIdOrUserref) -> Self {
        match value {
            TxIdOrUserref::TxId(uuid) => CancelOrderBy::TxId(OrderUuid(uuid)),
            TxIdOrUserref::Userref(userref) => CancelOrderBy::Userref(userref),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CancelOrderBatch {
    pub nonce: u64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub orders: Option<Vec<TxIdOrUserref>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub cl_ord_ids: Option<Vec<String>>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CancelOrderBatchResponse {
    pub count: usize,
}

pub async fn f(
    State(engine): State<OrderManagement>,
    State(users): State<crate::Users>,
    State(pg_pool): State<PgPool>,
    Extension(clerk): Extension<Clerk>,
    Json(CancelOrderBatch {
        nonce: _,
        orders,
        cl_ord_ids,
    }): Json<CancelOrderBatch>,
) -> Result<Json<CancelOrderBatchResponse>, (StatusCode, &'static str)> {
    // Build list of cancellation criteria
    let mut cancel_requests: Vec<CancelOrderBy> = Vec::new();

    // Add orders (txid/userref)
    if let Some(orders) = orders {
        cancel_requests.extend(orders.into_iter().map(CancelOrderBy::from));
    }

    // Add cl_ord_ids
    if let Some(cl_ord_ids) = cl_ord_ids {
        cancel_requests.extend(cl_ord_ids.into_iter().map(CancelOrderBy::ClientOrderId));
    }

    // Validate constraints
    if cancel_requests.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "must provide at least one order to cancel",
        ));
    }

    if cancel_requests.len() > 50 {
        return Err((
            StatusCode::BAD_REQUEST,
            "cannot cancel more than 50 orders at once",
        ));
    }

    match engine
        .cancel_order_batch(
            cancel_requests,
            users.to_user_pk(clerk.user_id(), pg_pool).await,
        )
        .await
    {
        Ok(count) => Ok(Json(CancelOrderBatchResponse { count })),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to cancel orders")),
    }
}
