use ap_actor::proc::TickerSnapshot;
use ap_actor::proc_router::ProcRouter;
use axum::Json;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use matching_engine::decimal::Decimal;
use std::collections::HashMap;
use tracing::Instrument;

fn default_asset_class() -> String {
    "forex".to_owned()
}

#[derive(Debug, serde::Deserialize)]
pub struct TickerRequest {
    /// Asset pair to get data for (optional, default: all tradeable exchange pairs)
    /// Example: XBTUSD
    pair: Option<String>,
    /// This parameter is required on requests for tokenized pairs, i.e. xstocks.
    /// **Possible values**: [`tokenized_asset`, `forex`]
    /// **Default value**: `forex`
    #[serde(default = "default_asset_class")]
    asset_class: String,
}

#[derive(Debug, serde::Serialize)]
pub struct AssetTickerInfo {
    a: [String; 3],
    b: [String; 3],
    c: [String; 2],
    v: [String; 2],
    p: [String; 2],
    t: [u64; 2],
    l: [String; 2],
    h: [String; 2],
    o: String,
}

#[derive(Debug, serde::Serialize)]
pub struct TickerResponse {
    error: Vec<String>,
    result: HashMap<String, AssetTickerInfo>,
}

/// Get ticker information for one or more asset pairs
pub async fn f(
    State(proc_router): State<ProcRouter>,
    Query(TickerRequest { pair, asset_class }): Query<TickerRequest>,
) -> Result<Json<TickerResponse>, (StatusCode, &'static str)> {
    let span = tracing::info_span!(
        "ticker",
        pair = pair.as_deref().unwrap_or("all"),
        asset_class = %asset_class
    );

    let processors: Vec<_> = match pair {
        Some(ref pair) => {
            let Some(handle) = proc_router
                .iter_active_processors()
                .find(|proc_handle| proc_handle.asset_pair_row.altname == *pair)
            else {
                return Err((StatusCode::NOT_FOUND, "asset pair not enabled"));
            };
            vec![handle]
        }
        None => proc_router.iter_active_processors().collect(),
    };

    let mut result = HashMap::new();

    for handle in processors {
        if !asset_class.is_empty()
            && asset_class != handle.asset_pair_row.aclass_base
            && asset_class != handle.asset_pair_row.aclass_quote
        {
            continue;
        }

        let snapshot = fetch_snapshot(&proc_router, handle)
            .instrument(tracing::trace_span!(
                "fetch_ticker_snapshot",
                base = %handle.asset_pair_row.base_asset,
                quote = %handle.asset_pair_row.quote_asset
            ))
            .await
            .map_err(|err| {
                tracing::warn!(?err, "ticker unavailable");
                (StatusCode::SERVICE_UNAVAILABLE, "ticker unavailable")
            })?;

        result.insert(
            handle.asset_pair_row.altname.clone(),
            convert_snapshot(snapshot),
        );
    }

    if result.is_empty() && pair.is_some() {
        return Err((StatusCode::NOT_FOUND, "asset pair not enabled"));
    }

    Ok(Json(TickerResponse {
        error: vec![],
        result,
    }))
}

async fn fetch_snapshot(
    proc_router: &ProcRouter,
    handle: &ap_actor::proc::ProcHandle,
) -> Result<TickerSnapshot, &'static str> {
    proc_router
        .ticker_snapshot(handle.base_quote.clone())
        .await
        .map_err(|_| "ticker snapshot rejected")
}

fn convert_snapshot(snapshot: TickerSnapshot) -> AssetTickerInfo {
    let zero = Decimal::ZERO;

    let last_price = snapshot.last_trade_price.unwrap_or(zero);
    let last_volume = snapshot.last_trade_volume.unwrap_or(zero);
    let open_price = snapshot.opening_price_today.unwrap_or(last_price);

    let best_bid = snapshot
        .best_bid
        .map(|d| (d.price, d.volume))
        .unwrap_or((zero, zero));
    let best_ask = snapshot
        .best_ask
        .map(|d| (d.price, d.volume))
        .unwrap_or((zero, zero));

    AssetTickerInfo {
        a: [
            best_ask.0.to_string(),
            best_ask.1.to_string(),
            best_ask.1.to_string(),
        ],
        b: [
            best_bid.0.to_string(),
            best_bid.1.to_string(),
            best_bid.1.to_string(),
        ],
        c: [last_price.to_string(), last_volume.to_string()],
        v: [
            snapshot.volume_today.to_string(),
            snapshot.volume_24h.to_string(),
        ],
        p: [
            snapshot.vwap_today.unwrap_or(zero).to_string(),
            snapshot.vwap_24h.unwrap_or(zero).to_string(),
        ],
        t: [snapshot.trades_today, snapshot.trades_24h],
        l: [
            snapshot.low_today.unwrap_or(zero).to_string(),
            snapshot.low_24h.unwrap_or(zero).to_string(),
        ],
        h: [
            snapshot.high_today.unwrap_or(zero).to_string(),
            snapshot.high_24h.unwrap_or(zero).to_string(),
        ],
        o: open_price.to_string(),
    }
}
