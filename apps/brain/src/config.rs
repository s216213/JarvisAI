use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub listen_addr: String,
    pub cors_origins: Vec<String>,
    pub groq_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub bootstrap_superadmin_email: Option<String>,
    pub bootstrap_superadmin_password: Option<String>,
    pub ws_ticket_ttl_secs: i64,
    pub access_token_ttl_secs: i64,
    pub refresh_token_ttl_secs: i64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            // Local default: file SQLite. Production/OCI: postgres://...
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./data/jarvis.db".into()),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| "dev-change-me-jarvis-secret".into()),
            listen_addr: env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            groq_api_key: env::var("GROQ_API_KEY").ok().filter(|s| !s.is_empty()),
            gemini_api_key: env::var("GEMINI_API_KEY").ok().filter(|s| !s.is_empty()),
            openrouter_api_key: env::var("OPENROUTER_API_KEY").ok().filter(|s| !s.is_empty()),
            bootstrap_superadmin_email: env::var("BOOTSTRAP_SUPERADMIN_EMAIL").ok(),
            bootstrap_superadmin_password: env::var("BOOTSTRAP_SUPERADMIN_PASSWORD").ok(),
            ws_ticket_ttl_secs: env::var("WS_TICKET_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            access_token_ttl_secs: env::var("ACCESS_TOKEN_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(900),
            refresh_token_ttl_secs: env::var("REFRESH_TOKEN_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60 * 60 * 24 * 30),
        })
    }
}
