use crate::middleware::clerk::ClerkUserId;
use ap_actor::VirtualUserId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default)]
pub struct Users(Arc<Mutex<HashMap<ClerkUserId, VirtualUserId>>>);

impl Users {
    pub async fn to_virtual_user_id<F, Fut>(
        &self,
        clerk_id: ClerkUserId,
        or_insert_with: F,
    ) -> ap_actor::VirtualUserId
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = VirtualUserId>,
    {
        let mut users = self.0.lock().await;
        match users.get(&clerk_id) {
            Some(user_id) => *user_id as _,
            None => {
                let id = or_insert_with().await;
                users.insert(clerk_id, id);
                id as _
            }
        }
    }
}
