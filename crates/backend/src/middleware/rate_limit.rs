use axum::extract::FromRef;
use axum::extract::FromRequestParts;
use axum::http::Uri;
use axum::http::request::Parts;
use futures::FutureExt as _;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

type BucketKey = Uri;

#[derive(Debug, Default, Clone, FromRef)]
pub struct RateLimitState(pub Arc<Mutex<HashMap<BucketKey, TokenBucket>>>);

#[derive(Debug)]
struct TokenBucket {
    n_tokens: u128,
    last_refill: Instant,
    refill_duration: Duration,
}

impl TokenBucket {
    fn refill(&mut self, num: u128) {
        let now = Instant::now();
        let duration_since = now.duration_since(self.last_refill);
        if duration_since >= self.refill_duration {
            self.n_tokens += (duration_since.as_millis() / self.refill_duration.as_millis());
            self.n_tokens = std::cmp::min(self.n_tokens, num);
            self.last_refill = now
                - Duration::from_millis(
                    (duration_since.as_millis() % self.refill_duration.as_millis()) as u64,
                );
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Limit<const NUM: u128, const PER: u128>(pub ());

impl<const NUM: u128, const PER: u128> Limit<NUM, PER> {
    async fn from_request_parts_check<S>(parts: &mut Parts, state: &S) -> Result<Self, ()>
    where
        RateLimitState: FromRef<S>,
        S: Send + Sync,
    {
        let RateLimitState(inner): RateLimitState = FromRef::from_ref(state);
        let mut inner = inner.lock().await;

        let key = BucketKey::from_request_parts(parts, state)
            .await
            .map_err(|_| ())?;

        let token_bucket = inner.entry(key).or_insert(TokenBucket {
            n_tokens: NUM,
            last_refill: Instant::now(),
            refill_duration: Duration::from_millis(PER as u64),
        });

        token_bucket.refill(NUM);
        if token_bucket.n_tokens > 0 {
            token_bucket.n_tokens -= 1;
            Ok(Self(()))
        } else {
            Err(())
        }
    }
}

impl<const NUM: u128, const PER: u128, S> FromRequestParts<S> for Limit<NUM, PER>
where
    RateLimitState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ();

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 S,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, Self::Rejection>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Limit::from_request_parts_check(parts, state).boxed()
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use axum::http::Uri;
    use axum::response::IntoResponse;

    use crate::middleware::rate_limit::Limit;

    // #[tokio::test]
    fn test_rate_limit() {
        async fn handler(Limit(()): Limit<1, { const { Duration::from_secs(1).as_millis() } }>) {}
    }
}
