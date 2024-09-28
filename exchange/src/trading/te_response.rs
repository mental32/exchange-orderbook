#![allow(missing_docs)]

use tokio::sync::oneshot;

use super::TradingEngineError;

#[must_use]
pub struct TeResponse<T, E = TradingEngineError>(pub oneshot::Receiver<Result<T, E>>);

impl<T, E> TeResponse<T, E> {
    pub async fn wait(self) -> Option<Result<T, E>> {
        self.0.await.ok()
    }
}
