use crate::conditional::ConditionalParameters;
use crate::svc::engine::EngineFacade;
use crate::svc::routes::trade_add_order::TradeAddOrder;
use crate::svc::routes::trade_cancel_order::TradeCancelOrder;
use axum::Extension;
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::response::IntoResponse;
use common_core::web::middleware::clerk::Clerk;
use futures::SinkExt;
use futures::StreamExt;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use tokio::sync::mpsc;
use tokio::time::Instant;

pub type SystemMsg = ();

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "method", content = "params"))]
enum WsRpc {
    #[cfg_attr(feature = "serde", serde(rename = "add_order"))]
    AddOrder {
        #[cfg_attr(feature = "serde", serde(flatten))]
        details: TradeAddOrder,
    },
    AmendOrder,
    #[cfg_attr(feature = "serde", serde(rename = "cancel_order"))]
    CancelOrder {
        #[cfg_attr(feature = "serde", serde(flatten))]
        details: TradeCancelOrder,
    },
    CancelAll,
    CancelAllOrdersAfter,
    BatchAdd,
    BatchCancel,
    EditOrder,
    Subscribe,
    Status,
    Heartbeat,
    Ping,
}

// #[cfg_attr(test, test)]
// fn test_de_ws_rpc() {
//     let st = serde_json::to_string_pretty(&WsRpc::AddOrder {
//         details: TradeAddOrder {
//             order_type: crate::orderbook::OrderType::StopLoss,
//             side: crate::orderbook::OrderSide::Sell,
//             quantity: rust_decimal::dec!(100),
//             symbol: "MATIC/USD".to_string(),
//             price: rust_decimal::dec!(10000),
//             conditional: None,
//             display_quantity: rust_decimal::dec!(0),
//             time_in_force: crate::timeinforce::TimeInForce::GoodTilCanceled,
//             stp: crate::self_trade_protection::SelfTradeProtection::CancelBoth,
//         },
//     })
//     .unwrap();
//     panic!("{st}");
// }

pub async fn f(
    State(engine): State<EngineFacade>,
    Extension(clerk): Extension<Clerk>,
    ws_upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    ws_upgrade
        .on_failed_upgrade(|error| {
            tracing::error!(?error, "websocket upgrade failed");
        })
        .on_upgrade(move |mut socket| async move {
            if let Err(err) = socket.send(Message::Ping(b"ping".to_vec())).await {
                tracing::error!(?err, "websocket ping failure");
                return; // "only thing we can do is drop the connection; there is no way to salvage the state machine anyway."
            }

            // TODO: send "status" message to new connection

            let (w_snd, mut w_rcv) = mpsc::channel(1);

            engine
                .inner
                .websocket_sessions
                .write()
                .await
                .push((clerk.user_id(), w_snd));

            let (sink, mut stream): (SplitSink<WebSocket, Message>, SplitStream<WebSocket>) =
                socket.split();

            loop {
                enum Either {
                    Left(Option<Result<Message, axum::Error>>),
                    Right(Option<SystemMsg>),
                }

                let either = tokio::select! {
                    usrmsg = stream.next() => Either::Left(usrmsg),
                    sysmsg = w_rcv.recv() => Either::Right(sysmsg),
                };

                match either {
                    Either::Left(usrmsg) => {
                        let time_in = Instant::now();

                        let msg_st = match usrmsg {
                            Some(Ok(Message::Binary(ref t))) => match std::str::from_utf8(&t) {
                                Ok(st) => st,
                                Err(err) => {
                                    tracing::warn!(?err, "binary message was not valid UTF-8");
                                    continue;
                                }
                            },
                            Some(Ok(Message::Text(ref t))) => t.as_str(),
                            Some(Ok(Message::Close(frame))) => {
                                tracing::info!(?frame, "user closed connection");
                                return;
                            }
                            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                            Some(Err(err)) => {
                                tracing::error!(?err, "error receiving message from user");
                                return;
                            }
                            None => {
                                tracing::error!("stream closed");
                                return;
                            }
                        };

                        #[derive(serde::Deserialize)]
                        struct WithReqId {
                            #[serde(flatten)]
                            rpc: WsRpc,
                            req_id: Option<usize>,
                        }

                        #[derive(serde::Deserialize)]
                        struct Method {
                            method: String,
                        }

                        let Method { method } = serde_json::from_str(&msg_st).expect("TODO");

                        let WithReqId { rpc, req_id } =
                            serde_json::from_str(&msg_st).expect("TODO");

                        match rpc {
                            WsRpc::AddOrder { .. } => todo!(),
                            WsRpc::AmendOrder => todo!(),
                            WsRpc::CancelOrder { .. } => todo!(),
                            WsRpc::CancelAll => todo!(),
                            WsRpc::CancelAllOrdersAfter => todo!(),
                            WsRpc::BatchAdd => todo!(),
                            WsRpc::BatchCancel => todo!(),
                            WsRpc::EditOrder => todo!(),
                            WsRpc::Subscribe => todo!(),
                            WsRpc::Status => todo!(),
                            WsRpc::Heartbeat => todo!(),
                            WsRpc::Ping => todo!(),
                        }

                        let time_out = Instant::now();

                        let response = serde_json::json!({
                            "method": method,
                            "result": {},
                            "error": {},
                            "success": true,
                            "req_id": req_id,
                            "time_in": time_out.elapsed().as_millis(),
                            "time_out": time_out.elapsed().as_millis()
                        });

                        sink.send(Message::Text(serde_json::to_string(&response).unwrap()))
                            .await
                            .unwrap();
                    }
                    Either::Right(sysmsg) => {
                        todo!("message from trading system to user")
                    }
                }
            }
        })
}
