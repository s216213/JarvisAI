use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderName {
    Groq,
    Gemini,
    Openrouter,
}

impl ProviderName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Groq => "groq",
            Self::Gemini => "gemini",
            Self::Openrouter => "openrouter",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "groq" => Some(Self::Groq),
            "gemini" => Some(Self::Gemini),
            "openrouter" => Some(Self::Openrouter),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderState {
    Ready,
    Cooling,
    Disabled,
}

#[derive(Debug, Default)]
struct ProviderStats {
    requests: AtomicU64,
    errors: AtomicU64,
    last_429: RwLock<Option<Instant>>,
    cooling_until: RwLock<Option<Instant>>,
    latency_ms_ema: RwLock<Option<f64>>,
    disabled: RwLock<bool>,
}

#[derive(Debug, Serialize)]
pub struct ProviderHealth {
    pub name: ProviderName,
    pub state: ProviderState,
    pub requests: u64,
    pub errors: u64,
    pub last_429_at: Option<String>,
    pub cooling_until: Option<String>,
    pub latency_ms_ema: Option<f64>,
}

pub struct QuotaGuardian {
    providers: HashMap<ProviderName, ProviderStats>,
    cooling_default: Duration,
}

impl Default for QuotaGuardian {
    fn default() -> Self {
        Self::new()
    }
}

impl QuotaGuardian {
    pub fn new() -> Self {
        let mut providers = HashMap::new();
        for name in [ProviderName::Groq, ProviderName::Gemini, ProviderName::Openrouter] {
            providers.insert(name, ProviderStats::default());
        }
        Self {
            providers,
            cooling_default: Duration::from_secs(60),
        }
    }

    pub fn is_available(&self, name: ProviderName) -> bool {
        let Some(stats) = self.providers.get(&name) else {
            return false;
        };
        if *stats.disabled.read() {
            return false;
        }
        match *stats.cooling_until.read() {
            Some(until) if Instant::now() < until => false,
            _ => true,
        }
    }

    pub fn mark_success(&self, name: ProviderName, latency_ms: u64) {
        if let Some(stats) = self.providers.get(&name) {
            stats.requests.fetch_add(1, Ordering::Relaxed);
            let mut ema = stats.latency_ms_ema.write();
            let sample = latency_ms as f64;
            *ema = Some(match *ema {
                Some(prev) => prev * 0.8 + sample * 0.2,
                None => sample,
            });
            *stats.cooling_until.write() = None;
        }
    }

    pub fn mark_error(&self, name: ProviderName, is_quota: bool) {
        if let Some(stats) = self.providers.get(&name) {
            stats.requests.fetch_add(1, Ordering::Relaxed);
            stats.errors.fetch_add(1, Ordering::Relaxed);
            if is_quota {
                *stats.last_429.write() = Some(Instant::now());
                *stats.cooling_until.write() = Some(Instant::now() + self.cooling_default);
            }
        }
    }

    pub fn set_disabled(&self, name: ProviderName, disabled: bool) {
        if let Some(stats) = self.providers.get(&name) {
            *stats.disabled.write() = disabled;
        }
    }

    pub fn retry_after_secs(&self) -> u64 {
        let now = Instant::now();
        self.providers
            .values()
            .filter_map(|s| *s.cooling_until.read())
            .filter(|u| *u > now)
            .map(|u| (u - now).as_secs().max(1))
            .min()
            .unwrap_or(30)
    }

    pub fn snapshot(&self) -> Vec<ProviderHealth> {
        let mut out = Vec::new();
        for name in [ProviderName::Groq, ProviderName::Gemini, ProviderName::Openrouter] {
            let stats = &self.providers[&name];
            let cooling = *stats.cooling_until.read();
            let state = if *stats.disabled.read() {
                ProviderState::Disabled
            } else if cooling.map(|u| Instant::now() < u).unwrap_or(false) {
                ProviderState::Cooling
            } else {
                ProviderState::Ready
            };
            out.push(ProviderHealth {
                name,
                state,
                requests: stats.requests.load(Ordering::Relaxed),
                errors: stats.errors.load(Ordering::Relaxed),
                last_429_at: stats.last_429.read().map(|t| format!("{t:?}")),
                cooling_until: cooling.map(|t| format!("{t:?}")),
                latency_ms_ema: *stats.latency_ms_ema.read(),
            });
        }
        out
    }
}
