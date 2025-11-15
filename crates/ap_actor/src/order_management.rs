use crate::VirtualUserId;
use crate::proc::AmendOrderArgs;
use crate::proc::CancelOrderByArgs;
use crate::proc::DescribeOrderArgs;
use crate::proc::Envelope;
use crate::proc::MsgError;
use crate::proc::MsgIn;
use crate::proc::MsgOut;
use crate::proc::OrderDescription;
use crate::proc::OrderDescriptor;
use crate::proc::OrderResult;
use crate::proc::ProcHandle;
use crate::user_profile::OpenOrder;
use crate::user_profile::UserProfile;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use matching_engine::asset_code::SymbolVocabulary;
use matching_engine::asset_pair::BaseQuote;
use matching_engine::order_ticket::OrderTicket;
use matching_engine::order_uuid::OrderUuid;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlaceOrderArgs {
    pub base_quote: BaseQuote,
    pub user_id: VirtualUserId,
    pub order_uuid: OrderUuid,
    pub order_details: OrderTicket,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CancelOrderBy {
    TxId(OrderUuid),
    Userref(u32),
    ClientOrderId(String),
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    Currency,
    TokenizedAsset,
}

pub type CancelOrderError = ((), &'static str);

#[derive(Debug)]
struct Inner {
    asset_processors: Vec<ProcHandle>,
    tracking: HashMap<VirtualUserId, UserProfile>,
}

#[derive(Debug, Clone)]
pub struct OrderManagement {
    pub symbol_vocabulary: SymbolVocabulary,
    inner: Arc<Inner>,
}

impl OrderManagement {
    pub fn new(symbol_vocabulary: SymbolVocabulary, asset_processors: Vec<ProcHandle>) -> Self {
        Self {
            symbol_vocabulary,
            inner: Arc::new(Inner {
                asset_processors,
                tracking: Default::default(),
            }),
        }
    }

    fn get_asset_processor_channel(
        &self,
        (base, quote): &BaseQuote,
    ) -> Option<mpsc::Sender<Envelope>> {
        self.inner
            .asset_processors
            .iter()
            .find(|i| i.base_quote.0 == *base && i.base_quote.1 == *quote)
            .map(|i| i.mpsc_sender.clone())
    }
}

impl OrderManagement {
    pub fn is_pair_enabled(&self, symbol: &str) -> Option<BaseQuote> {
        self.inner
            .asset_processors
            .iter()
            .find(|i| i.base_quote.0.as_str() == symbol)
            .map(|i| i.base_quote.clone())
    }

    pub fn get_active_processors(&self) -> impl Iterator<Item = &ProcHandle> {
        self.inner.asset_processors.iter()
    }

    pub fn open_orders_for(&self, user_id: &VirtualUserId) -> Vec<OpenOrder> {
        self.inner
            .tracking
            .get(user_id)
            .map(|profile| profile.open_orders.clone())
            .unwrap_or_default()
    }

    pub async fn cancel_order(
        &self,
        cancel_order_by: CancelOrderBy,
        user_id: VirtualUserId,
    ) -> Result<(Vec<uuid::Uuid>, Vec<(uuid::Uuid, String)>), CancelOrderError> {
        let mut pairs_to_contact = HashMap::new();

        let mut success = vec![];
        let mut failed = vec![];

        let pending = FuturesUnordered::new();

        for open_order in self
            .inner
            .tracking
            .get(&user_id)
            .map(|profile| profile.isolate_orders_for_cancel(&cancel_order_by))
            .unwrap_or_default()
            .into_iter()
        {
            pairs_to_contact
                .entry(open_order.base_quote)
                .or_insert(vec![])
                .push(open_order.order_uuid.clone());
        }

        for (base_quote, ids) in pairs_to_contact {
            let Some(ap_snd) = self.get_asset_processor_channel(&base_quote) else {
                tracing::error!(
                    base = ?base_quote.0.as_str(),
                    quote = ?base_quote.1.as_str(),
                    "attempted to cancel order on non-existent asset pair"
                );
                failed.extend(
                    ids.into_iter()
                        .map(|id| (id.0, "Asset pair not found".to_owned())),
                );
                continue;
            };

            let (msg_snd, msg_rcv) = oneshot::channel();
            let msg_in = MsgIn::CancelOrderBy(CancelOrderByArgs {
                user_id: user_id.clone(),
                cancel_order_by: cancel_order_by.clone(),
            });

            pending.push(async move {
                let () = ap_snd
                    .send((msg_snd, msg_in))
                    .await
                    .map_err(|_| ids.clone())?;

                match msg_rcv.await.map_err(|_| ids.clone())? {
                    Ok(MsgOut::OrderCancelled { success, failed }) => Ok((success, failed)),
                    Ok(_t) => unreachable!("bug: these outputs should never be received"),
                    Err(_) => Err(ids),
                }
            });
        }

        tokio::pin!(pending);

        while let Some(res) = pending.next().await {
            let (s, f) = match res {
                Ok((s, f)) => (s, f),
                Err(f) => (vec![], f),
            };

            success.extend(s.into_iter().map(|id| id.0));

            failed.extend(
                f.into_iter()
                    .map(|id| (id.0, "Failed to cancel order".to_owned())), // TODO: the error message is generic i am aware. will fix later.
            )
        }

        Ok((success, failed))
    }

    pub async fn amend_order(
        &self,
        order_uuid: OrderUuid,
        user_id: VirtualUserId,
        amend_args: AmendOrderArgs,
    ) -> Result<OrderUuid, MsgError> {
        // Find the order to determine its asset pair
        let base_quote = self
            .inner
            .tracking
            .get(&user_id)
            .and_then(|profile| {
                profile
                    .open_orders
                    .iter()
                    .find(|o| o.order_uuid == order_uuid)
                    .map(|o| o.base_quote.clone())
            })
            .ok_or(MsgError::OrderNotFound)?;

        let Some(ap_snd) = self.get_asset_processor_channel(&base_quote) else {
            tracing::error!(
                base = ?base_quote.0.as_str(),
                quote = ?base_quote.1.as_str(),
                "attempted to amend order on non-existent asset pair"
            );
            return Err(todo!(" (StatusCode::NOT_FOUND, \"Asset pair not found\") "));
        };

        let (msg_snd, msg_rcv) = oneshot::channel();
        let msg_in = MsgIn::AmendOrder(amend_args);

        ap_snd
            .send((msg_snd, msg_in))
            .await
            .expect("failed to send message to actor");

        match msg_rcv.await.expect("Failed to receive response") {
            Ok(MsgOut::OrderAmended { order_uuid }) => Ok(order_uuid),
            Ok(_) => unreachable!("bug: unexpected response type"),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn place_order(
        &self,
        base_quote: BaseQuote,
        user_id: VirtualUserId,
        order_details: OrderTicket,
    ) -> Result<OrderUuid, MsgError> {
        let Some(ap_snd) = self.get_asset_processor_channel(&base_quote) else {
            tracing::error!(
                base = ?base_quote.0.as_str(),
                quote = ?base_quote.1.as_str(),
                "attempted to place order on non-existent asset pair"
            );
            return Err(todo!(" StatusCode::NOT_FOUND, \"Asset pair not found\""));
        };

        let order_uuid = OrderUuid(uuid::Uuid::new_v4());

        let (msg_snd, msg_rcv) = oneshot::channel();
        let msg_in = MsgIn::PlaceOrder(PlaceOrderArgs {
            base_quote,
            user_id: user_id.clone(),
            order_uuid,
            order_details,
        });

        if let Err(err) = ap_snd.send((msg_snd, msg_in)).await {
            tracing::error!(
                ?err,
                "error sending place order message to asset pair actor"
            );
            panic!("error sending place order message to asset pair actor");
        }

        match msg_rcv.await.unwrap()? {
            MsgOut::OrderPlaced => {
                tracing::info!(?order_uuid, "order placed");
                Ok(order_uuid.clone())
            }
            MsgOut::OrderValidated => todo!(),
            MsgOut::OrderCancelled { .. }
            | MsgOut::OrderAmended { .. }
            | MsgOut::OrderSnapshot { .. }
            | MsgOut::StatusChanged(_)
            | MsgOut::ShutdownAcknowledged => {
                unreachable!("bug: these outputs should never be received")
            }
        }
    }

    pub async fn place_order_batch(
        &self,
        base_quote: BaseQuote,
        user_id: VirtualUserId,
        orders: Vec<matching_engine::order_ticket::OrderTicket>,
    ) -> Result<Vec<OrderResult>, MsgError> {
        // Place orders sequentially, collecting individual results
        // Individual funding/engine failures don't stop the batch
        let mut results = Vec::with_capacity(orders.len());
        for order_details in orders {
            let descr = OrderDescription {
                order: format!(
                    "{:?} {} @ {:?} {}",
                    order_details.side,
                    order_details.volume,
                    order_details.order_type,
                    order_details.price.amount.to_string()
                ),
            };
            match self
                .place_order(base_quote.clone(), user_id, order_details)
                .await
            {
                Ok(order_uuid) => {
                    results.push(OrderResult {
                        txid: Some(order_uuid.0),
                        descr,
                        error: None,
                    });
                }
                Err(err) => {
                    tracing::warn!(?err, "order failed at engine submission in batch");
                    // Individual order failed, but continue processing remainder
                    results.push(OrderResult {
                        txid: None,
                        descr,
                        error: Some(err.to_string()),
                    });
                }
            }
        }
        Ok(results)
    }

    pub async fn cancel_order_batch(
        &self,
        cancel_requests: Vec<CancelOrderBy>,
        user_id: VirtualUserId,
    ) -> Result<usize, CancelOrderError> {
        // Cancel orders in parallel using FuturesUnordered
        let pending = FuturesUnordered::new();
        for cancel_by in cancel_requests {
            let self_clone = self.clone();
            let user_id = user_id.clone();
            pending.push(async move { self_clone.cancel_order(cancel_by, user_id).await });
        }
        tokio::pin!(pending);
        let mut total_count = 0;
        while let Some(result) = pending.next().await {
            match result {
                Ok((success, _failed)) => {
                    // Count successful cancellations
                    total_count += success.len();
                }
                Err(_) => {
                    // Individual cancellation errors don't stop the batch
                    // Just continue processing
                }
            }
        }
        Ok(total_count)
    }

    pub async fn describe_order(
        &self,
        base_quote: BaseQuote,
        user_id: VirtualUserId,
        order_uuid: OrderUuid,
    ) -> Result<OrderDescriptor, MsgError> {
        let Some(ap_snd) = self.get_asset_processor_channel(&base_quote) else {
            return Err(MsgError::OrderNotFound);
        };
        let (msg_snd, msg_rcv) = oneshot::channel();
        let msg_in = MsgIn::DescribeOrder(DescribeOrderArgs {
            user_id,
            order_uuid,
        });
        ap_snd
            .send((msg_snd, msg_in))
            .await
            .map_err(|_| MsgError::OrderNotFound)?;
        match msg_rcv.await.map_err(|_| MsgError::OrderNotFound)? {
            Ok(MsgOut::OrderSnapshot { descriptor }) => Ok(descriptor),
            Ok(_) => unreachable!("bug: unexpected response type"),
            Err(err) => Err(err),
        }
    }

    pub async fn describe_order_any(
        &self,
        user_id: VirtualUserId,
        order_uuid: OrderUuid,
    ) -> Result<(BaseQuote, OrderDescriptor), MsgError> {
        if let Some(open_order) = self
            .open_orders_for(&user_id)
            .into_iter()
            .find(|order| order.order_uuid == order_uuid)
        {
            let descriptor = self
                .describe_order(open_order.base_quote.clone(), user_id, order_uuid)
                .await?;
            return Ok((open_order.base_quote, descriptor));
        }
        for info in &self.inner.asset_processors {
            let (msg_snd, msg_rcv) = oneshot::channel();
            let msg_in = MsgIn::DescribeOrder(DescribeOrderArgs {
                user_id,
                order_uuid,
            });
            if info
                .mpsc_sender
                .clone()
                .send((msg_snd, msg_in))
                .await
                .is_err()
            {
                continue;
            }
            match msg_rcv.await {
                Ok(Ok(MsgOut::OrderSnapshot { descriptor })) => {
                    return Ok((info.base_quote.clone(), descriptor));
                }
                Ok(Ok(_)) => continue,
                Ok(Err(err)) => {
                    if matches!(err, MsgError::OrderNotFound) {
                        continue;
                    } else {
                        return Err(err);
                    }
                }
                Err(_) => continue,
            }
        }
        Err(MsgError::OrderNotFound)
    }
}
