use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub user_id: Uuid,
    pub platform: String,
    pub capabilities: Vec<String>,
    pub label: Option<String>,
    pub online: bool,
    pub last_seen: DateTime<Utc>,
}

#[derive(Clone)]
struct LiveDevice {
    info: DeviceInfo,
    tx: mpsc::UnboundedSender<String>,
}

pub struct DeviceRegistry {
    devices: DashMap<String, LiveDevice>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: DashMap::new(),
        }
    }

    pub fn register(
        &self,
        info: DeviceInfo,
        tx: mpsc::UnboundedSender<String>,
    ) {
        self.devices.insert(
            info.device_id.clone(),
            LiveDevice { info, tx },
        );
    }

    pub fn unregister(&self, device_id: &str) {
        self.devices.remove(device_id);
    }

    pub fn heartbeat(&self, device_id: &str) {
        if let Some(mut d) = self.devices.get_mut(device_id) {
            d.info.last_seen = Utc::now();
            d.info.online = true;
        }
    }

    pub fn list_for_user(&self, user_id: Uuid) -> Vec<DeviceInfo> {
        self.devices
            .iter()
            .filter(|e| e.info.user_id == user_id)
            .map(|e| e.info.clone())
            .collect()
    }

    pub fn find_for_capability(&self, user_id: Uuid, capability: &str) -> Option<DeviceInfo> {
        self.devices
            .iter()
            .find(|e| {
                e.info.user_id == user_id
                    && e.info.online
                    && e.info.capabilities.iter().any(|c| c == capability)
            })
            .map(|e| e.info.clone())
    }

    pub fn send_to_device(&self, device_id: &str, payload: String) -> bool {
        if let Some(d) = self.devices.get(device_id) {
            d.tx.send(payload).is_ok()
        } else {
            false
        }
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
