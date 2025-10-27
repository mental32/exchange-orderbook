use crate::middleware::clerk::Clerk;
use ap_actor::order_management::OrderManagement;
use ap_actor::proc::BroadcastEvent;
use axum::Extension;
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::response::IntoResponse;
use futures::SinkExt as _;
use futures::StreamExt as _;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use tokio::sync::mpsc;
use tokio::time::Instant;

pub mod add_order;
pub mod amend_order;
pub mod batch_add;
pub mod batch_cancel;
pub mod cancel_all;
pub mod cancel_on_disconnect;
pub mod cancel_order;
pub mod edit_order;
pub mod subscribe;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "method", content = "params"))]
enum WsRpc {
    #[cfg_attr(feature = "serde", serde(rename = "add_order"))]
    AddOrder {
        #[cfg_attr(feature = "serde", serde(flatten))]
        details: add_order::AddOrder,
    },
    AmendOrder,
    #[cfg_attr(feature = "serde", serde(rename = "cancel_order"))]
    CancelOrder {
        #[cfg_attr(feature = "serde", serde(flatten))]
        details: cancel_order::CancelOrder,
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

async fn socket_loop(
    mut socket: WebSocket,
    order_management: OrderManagement,
    Clerk { user: clerk_user }: Clerk,
) -> anyhow::Result<()> {
    socket
        .send(Message::Text(
            serde_json::to_string(&serde_json::json!({
                "channel": "status",
                "type": "update",
                "data": [
                    {
                        "system": "online", // TODO: possible values are: online, cancel_only, maintenance, post_only
                        "api_version": "v2",
                        "connection_id": 0,
                        "version": "2.0.0"
                    }
                ]
            }))
            .unwrap(),
        ))
        .await
        .unwrap();

    let (_w_snd, mut w_rcv) = mpsc::channel(1);

    let (mut sink, mut stream): (SplitSink<WebSocket, Message>, SplitStream<WebSocket>) =
        socket.split();

    loop {
        enum Either {
            Left(Option<Result<Message, axum::Error>>),
            Right(Option<BroadcastEvent>),
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
                        anyhow::bail!("user closed connection")
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                    Some(Err(err)) => {
                        tracing::error!(?err, "error receiving message from user");
                        anyhow::bail!("error receiving message from user")
                    }
                    None => {
                        tracing::error!("stream closed");
                        anyhow::bail!("stream closed")
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

                let WithReqId { rpc: _, req_id } = serde_json::from_str(&msg_st).expect("TODO");

                // match rpc {
                //     WsRpc::AddOrder { .. } => todo!(),
                //     WsRpc::AmendOrder => todo!(),
                //     WsRpc::CancelOrder { .. } => todo!(),
                //     WsRpc::CancelAll => todo!(),
                //     WsRpc::CancelAllOrdersAfter => todo!(),
                //     WsRpc::BatchAdd => todo!(),
                //     WsRpc::BatchCancel => todo!(),
                //     WsRpc::EditOrder => todo!(),
                //     WsRpc::Subscribe => todo!(),
                //     WsRpc::Status => todo!(),
                //     WsRpc::Heartbeat => todo!(),
                //     WsRpc::Ping => todo!(),
                // }

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
            Either::Right(_sysmsg) => {
                todo!("message from trading system to user")
            }
        }
    }
}

pub async fn f(
    State(order_management): State<OrderManagement>,
    Extension(clerk): Extension<Clerk>,
    websocket_upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    websocket_upgrade
        .on_failed_upgrade(|error| {
            tracing::error!(?error, "websocket upgrade failed");
        })
        .on_upgrade(move |socket| async move {
            if let Err(err) = socket_loop(socket, order_management, clerk).await {
                tracing::error!(?err, "websocket failure");
                return;
            }
        })
}
