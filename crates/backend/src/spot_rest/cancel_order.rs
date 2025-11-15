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
pub enum TxId {
    OrderIdentifier(uuid::Uuid),
    Userref(u32),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeCancelOrder {
    pub nonce: u64,
    pub txid: Option<TxId>,
    pub cl_ord_id: Option<String>,
}

#[cfg_attr(test, test)]
#[cfg_attr(not(test), allow(dead_code))]
fn test_de_trade_cancel_order() {
    let json =
        r#"{"nonce": 123, "txid": "6b9a6095-0838-40d8-8ecb-7555c1cf7ff2", "cl_ord_id": "123"}"#;
    let order = serde_json::from_str::<TradeCancelOrder>(json).unwrap();
    assert_eq!(order.nonce, 123);
    assert!(matches!(order.txid, Some(TxId::OrderIdentifier(_))));
    assert_eq!(order.cl_ord_id, Some("123".to_owned()));
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeCancelOrderResponse {
    #[cfg_attr(feature = "serde", serde(rename = "c"))]
    cancelled: Vec<uuid::Uuid>,
    #[cfg_attr(feature = "serde", serde(rename = "f"))]
    failed: Vec<(uuid::Uuid, String)>,
}

pub async fn f(
    State(engine): State<OrderManagement>,
    State(users): State<crate::Users>,
    State(pg_pool): State<PgPool>,
    Extension(clerk): Extension<Clerk>,
    Json(TradeCancelOrder {
        nonce: _,
        txid,
        cl_ord_id,
    }): Json<TradeCancelOrder>,
) -> Result<Json<TradeCancelOrderResponse>, (StatusCode, &'static str)> {
    let cancel_order_by = match (cl_ord_id, txid) {
        (None, None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Must provide either txid or cl_ord_id",
            ));
        }
        (None, Some(TxId::Userref(n))) => CancelOrderBy::Userref(n),
        (None, Some(TxId::OrderIdentifier(id))) => CancelOrderBy::TxId(OrderUuid(id)),
        (Some(st), None) => CancelOrderBy::ClientOrderId(st),
        (Some(_), Some(_)) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Cannot specify both txid and cl_ord_id",
            ));
        }
    };

    match engine
        .cancel_order(
            cancel_order_by,
            users.to_user_pk(clerk.user_id(), pg_pool).await,
        )
        .await
    {
        Ok((cancelled, failed)) => Ok(Json(TradeCancelOrderResponse { cancelled, failed })),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to cancel order")),
    }
}

#[cfg(all(feature = "serde", test))]
mod test_serde {
    use crate::spot_rest::cancel_order::TxId;

    #[test]
    fn test_de_txid() {
        let json = "\"6b9a6095-0838-40d8-8ecb-7555c1cf7ff2\"";
        let txid = serde_json::from_str::<TxId>(json).unwrap();
        assert!(matches!(txid, TxId::OrderIdentifier(_)));

        let json = r#"123123"#;
        let txid = serde_json::from_str::<TxId>(json).unwrap();
        assert!(matches!(txid, TxId::Userref(_)));
    }
}
