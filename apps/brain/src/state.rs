use std::sync::Arc;

use dashmap::DashMap;
use reqwest::Client;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    config::Config,
    db::DbPool,
    devices::registry::DeviceRegistry,
    llm::quota::QuotaGuardian,
    rate_limit::RateLimiter,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Arc<Config>,
    pub http: Client,
    pub quota: Arc<QuotaGuardian>,
    pub devices: Arc<DeviceRegistry>,
    pub user_events: Arc<DashMap<Uuid, broadcast::Sender<String>>>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(pool: DbPool, config: Config) -> Self {
        Self {
            pool,
            config: Arc::new(config),
            http: Client::new(),
            quota: Arc::new(QuotaGuardian::new()),
            devices: Arc::new(DeviceRegistry::new()),
            user_events: Arc::new(DashMap::new()),
            rate_limiter: Arc::new(RateLimiter::new()),
        }
    }

    pub fn subscribe_user(&self, user_id: Uuid) -> broadcast::Receiver<String> {
        self.user_events
            .entry(user_id)
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(64);
                tx
            })
            .subscribe()
    }

    pub fn publish_user(&self, user_id: Uuid, payload: String) {
        if let Some(tx) = self.user_events.get(&user_id) {
            let _ = tx.send(payload);
        }
    }
}
