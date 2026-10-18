mod admin;
mod auth;
mod config;
mod db;
mod devices;
mod error;
mod flags;
mod llm;
mod memory;
mod persona;
mod rate_limit;
mod session;
mod state;
mod tools;

use axum::{
    Json, Router,
    http::{HeaderValue, Method},
    middleware,
    routing::get,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    admin::admin_router,
    auth::{auth_router, bootstrap_superadmin},
    config::Config,
    devices::device_ws_handler,
    flags::flags_router,
    memory::memory_router,
    persona::persona_router,
    rate_limit::llm_user_rate_limit,
    session::{chat_ws_handler, threads_router},
    state::AppState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "jarvis_brain=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let pool = db::connect(&config.database_url).await?;
    let state = AppState::new(pool, config.clone());
    bootstrap_superadmin(&state).await?;

    let cors = build_cors(&config.cors_origins);

    let app = Router::new()
        .route("/health", get(health))
        .nest("/v1/auth", auth_router())
        .nest("/v1/threads", threads_router())
        .nest("/v1/memory", memory_router())
        .nest("/v1/persona", persona_router())
        .nest("/v1/admin/flags", flags_router())
        .nest("/v1/admin", admin_router())
        .route("/v1/chat/ws", get(chat_ws_handler))
        .route("/v1/device/ws", get(device_ws_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            llm_user_rate_limit,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    tracing::info!(addr = %config.listen_addr, "Jarvis brain listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true, "service": "jarvis-brain" }))
}

fn build_cors(origins: &[String]) -> CorsLayer {
    let allow = if origins.iter().any(|o| o == "*") {
        AllowOrigin::any()
    } else {
        let list: Vec<HeaderValue> = origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        AllowOrigin::list(list)
    };

    CorsLayer::new()
        .allow_origin(allow)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true)
}
