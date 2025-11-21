use ap_actor::proc_router::AssetClass;
use ap_actor::proc_router::ProcRouter;
use axum::extract::Json;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use futures::StreamExt as _;
use futures::TryStreamExt as _;
use matching_engine::decimal::Decimal;
use sqlx::prelude::FromRow;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::middleware::either::Either;
use crate::middleware::either::Parts;

mod asset {
    pub fn serialize<S>(value: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::Serialize as _;

        value.join(",").to_string().serialize(serializer)
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetsRequest {
    #[serde(with = "self::asset", default)]
    asset: Vec<String>,
    aclass: Option<AssetClass>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetInfo {
    /// Asset Class
    pub aclass: String,
    /// Alternative name
    pub altname: String,
    /// Number of decimal places for record keeping amounts of this asset
    pub decimals: u8,
    /// Number of decimal places shown for display purposes in frontends
    pub display_decimals: u8,
    /// Valuation as margin collateral (if applicable)
    pub collateral_value: Option<Decimal>,
    /// Status of asset.
    /// Possible values: `enabled`, `deposit_only`, `withdrawl_only` `funding_temporarily_disabled`
    pub status: String,
}

#[axum::debug_handler]
pub async fn f(
    State(pg_pool): State<sqlx::PgPool>,
    body: Either<Json<AssetsRequest>, Parts<Query<AssetsRequest>>>,
) -> Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let AssetsRequest { asset, aclass } = match body {
        Either::Left(Json(t)) | Either::Right(Parts(Query(t))) => t,
    };

    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT id, asset_class, alternate_name, decimals, display_decimals, collateral_value, status FROM t_assets WHERE ",
    );

    if let Some(aclass) = aclass {
        let aclass_str = match aclass {
            AssetClass::Currency => "currency",
            AssetClass::TokenizedAsset => "tokenized_asset",
        };
        query_builder.push("asset_class = ");
        query_builder.push_bind(aclass_str);
        query_builder.push(" AND ");
    }

    match asset.as_slice() {
        [] => {
            // all assets
            query_builder.push("1 = 1");
        }
        [one] => {
            query_builder.push("(id = ");
            query_builder.push_bind(one);
            query_builder.push(" OR alternate_name = ");
            query_builder.push_bind(one);
            query_builder.push(")");
        }
        many => {
            query_builder.push("(");
            let mut separated = query_builder.push("id IN (").separated(", ");
            for name in many {
                separated.push_bind(name);
            }
            separated.push_unseparated(") OR ");

            let mut separated = query_builder.push("alternate_name IN (").separated(", ");
            for name in many {
                separated.push_bind(name);
            }
            separated.push_unseparated("))");
        }
    }

    #[derive(Debug, FromRow)]
    struct AssetInfoRow {
        id: String,
        asset_class: String,
        alternate_name: String,
        decimals: i32,
        display_decimals: i32,
        status: String,
        collateral_value: Option<Decimal>,
    }

    let stream = query_builder
        .build_query_as::<AssetInfoRow>()
        .fetch(&pg_pool);
    tokio::pin!(stream);
    let mut result = vec![];
    let mut error = vec![];
    while let Some(row) = stream.next().await {
        let row = match row {
            Ok(row) => row,
            Err(err) => {
                error.push(err.to_string());
                continue;
            }
        };
        let asset_info = AssetInfo {
            aclass: row.asset_class,
            altname: row.id,
            decimals: row.decimals as _,
            display_decimals: row.display_decimals as _,
            collateral_value: row.collateral_value as _,
            status: row.status,
        };
        result.push(asset_info);
    }

    Ok(Json(serde_json::json!({
        "error": error,
        "result": result,
    })))
}
