use crate::middleware::clerk::ClerkUserId;
use ap_actor::VirtualUserId;
use futures::FutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
pub struct Users(Arc<RwLock<HashMap<ClerkUserId, VirtualUserId>>>);

impl Users {
    pub async fn to_user_pk(&self, clerk_id: ClerkUserId, pg_pool: sqlx::PgPool) -> VirtualUserId {
        {
            if let Some(t) = self.0.read().await.get(&clerk_id).copied() {
                return t;
            }
        }

        let mut hash_map = self.0.write().await;

        // check again since some other writer could have populated the map before we acquired the write lock
        if let Some(t) = hash_map.get(&clerk_id).copied() {
            return t;
        }

        let user_pk = sqlx::query!("SELECT id FROM t_user_data WHERE clerk = $1", clerk_id.0)
            .fetch_one(&pg_pool)
            .await
            .expect("this is a panicking api")
            .id;

        hash_map.insert(clerk_id, user_pk);

        user_pk
    }
}
