use crate::middleware::either::Either;
use crate::middleware::either::Parts;
use ap_actor::order_management;
use ap_actor::order_management::AssetClass;
use ap_actor::order_management::OrderManagement;
use ap_actor::proc::ProcHandle;
use axum::extract::Json;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use futures::StreamExt as _;
use matching_engine::asset_pair::AssetPairRow;
use matching_engine::decimal::Decimal;
use sqlx::prelude::FromRow;
use std::collections::BTreeMap;

mod pair {
    pub fn serialize<S>(value: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::Serialize as _;

        value.join(",").serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;

        let st: String = String::deserialize(deserializer)?;

        let parts = st
            .split(",")
            .filter_map(|st| {
                let trimmed = st.trim();
                match trimmed {
                    "" => None,
                    _ => Some(trimmed.to_owned()),
                }
            })
            .collect::<Vec<_>>();

        Ok(parts)
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[serde(rename_all = "lowercase")]
enum InfoType {
    Info,
    Leverage,
    Fees,
    Margin,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetPairsRequest {
    #[serde(with = "self::pair", default)]
    pair: Vec<String>,
    aclass_base: Option<AssetClass>,
    info: Option<InfoType>,
    #[allow(dead_code)]
    country_code: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetPairInfo {
    pub altname: String,
    pub wsname: String,
    pub aclass_base: String,
    pub base: String,
    pub aclass_quote: String,
    pub quote: String,
    pub pair_decimals: i32,
    pub cost_decimals: i32,
    pub lot_decimals: i32,
    pub lot: String,
    pub lot_multiplier: i32,
    pub leverage_buy: Vec<i32>,
    pub leverage_sell: Vec<i32>,
    pub fees: serde_json::Value,
    pub fees_maker: serde_json::Value,
    pub fee_volume_currency: String,
    pub margin_call: i32,
    pub margin_stop: i32,
    pub ordermin: String,
    pub costmin: String,
    pub tick_size: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_position_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_position_limit: Option<i32>,
}

pub async fn f(
    State(pg_pool): State<sqlx::PgPool>,
    State(order_management): State<OrderManagement>,
    body: Either<Json<AssetPairsRequest>, Parts<Query<AssetPairsRequest>>>,
) -> Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let AssetPairsRequest {
        pair,
        aclass_base,
        info,
        ..
    } = match body {
        Either::Left(Json(t)) | Either::Right(Parts(Query(t))) => t,
    };

    let normalized_pairs = order_management
        .get_active_processors()
        .map(|proc_handle: ProcHandle| todo!())
        .collect();

    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT id, base_asset, quote_asset, altname, wsname, status, \
         min_order_size, max_order_size, price_tick_size, quantity_tick_size, \
         pair_decimals, cost_decimals, lot_decimals, lot, lot_multiplier, \
         aclass_base, aclass_quote, leverage_buy, leverage_sell, \
         fees, fees_maker, fee_volume_currency, margin_call, margin_stop, \
         costmin, tick_size, long_position_limit, short_position_limit \
         FROM t_trading_asset_pairs WHERE ",
    );

    if let Some(aclass) = aclass_base {
        let aclass_str = match aclass {
            AssetClass::Currency => "currency",
            AssetClass::TokenizedAsset => "tokenized_asset",
        };
        query_builder.push("aclass_base = ");
        query_builder.push_bind(aclass_str);
        query_builder.push(" AND ");
    }

    match normalized_pairs.as_slice() {
        [] => {
            // all pairs
            query_builder.push("1 = 1");
        }
        [(base, quote)] => {
            query_builder.push("(base_asset = ");
            query_builder.push_bind(base);
            query_builder.push(" AND quote_asset = ");
            query_builder.push_bind(quote);
            query_builder.push(")");
        }
        many => {
            query_builder.push("(");
            let mut first = true;
            for (base, quote) in many {
                if !first {
                    query_builder.push(" OR ");
                }
                query_builder.push("(base_asset = ");
                query_builder.push_bind(base);
                query_builder.push(" AND quote_asset = ");
                query_builder.push_bind(quote);
                query_builder.push(")");
                first = false;
            }
            query_builder.push(")");
        }
    }

    let stream = query_builder
        .build_query_as::<AssetPairRow>()
        .fetch(&pg_pool);
    tokio::pin!(stream);

    let mut result = BTreeMap::new();
    let mut errors = vec![];

    while let Some(row) = stream.next().await {
        let row = match row {
            Ok(row) => row,
            Err(err) => {
                errors.push(err.to_string());
                continue;
            }
        };

        let pair_info = AssetPairInfo {
            altname: row.altname.clone(),
            wsname: row.wsname,
            aclass_base: row.aclass_base,
            base: row.base_asset,
            aclass_quote: row.aclass_quote,
            quote: row.quote_asset,
            pair_decimals: row.pair_decimals,
            cost_decimals: row.cost_decimals,
            lot_decimals: row.lot_decimals,
            lot: row.lot,
            lot_multiplier: row.lot_multiplier,
            leverage_buy: row.leverage_buy,
            leverage_sell: row.leverage_sell,
            fees: row.fees,
            fees_maker: row.fees_maker,
            fee_volume_currency: row.fee_volume_currency,
            margin_call: row.margin_call,
            margin_stop: row.margin_stop,
            ordermin: row.min_order_size.to_string(),
            costmin: row.costmin.to_string(),
            tick_size: row.tick_size.to_string(),
            status: row.status,
            long_position_limit: row.long_position_limit,
            short_position_limit: row.short_position_limit,
        };

        // Filter by info parameter
        let filtered_info = match info {
            Some(InfoType::Leverage) => serde_json::json!({
                "leverage_buy": pair_info.leverage_buy,
                "leverage_sell": pair_info.leverage_sell,
            }),
            Some(InfoType::Fees) => serde_json::json!({
                "fees": pair_info.fees,
                "fees_maker": pair_info.fees_maker,
                "fee_volume_currency": pair_info.fee_volume_currency,
            }),
            Some(InfoType::Margin) => serde_json::json!({
                "margin_call": pair_info.margin_call,
                "margin_stop": pair_info.margin_stop,
            }),
            None | Some(InfoType::Info) => serde_json::to_value(&pair_info).unwrap(),
        };

        result.insert(row.altname, filtered_info);
    }

    Ok(Json(serde_json::json!({
        "error": errors,
        "result": result,
    })))
}
