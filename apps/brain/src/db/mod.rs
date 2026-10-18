use std::path::Path;

use anyhow::Context;

#[cfg(feature = "db-postgres")]
use sqlx::postgres::PgPoolOptions;
#[cfg(feature = "db-sqlite")]
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[cfg(feature = "db-postgres")]
pub type DbPool = sqlx::PgPool;
#[cfg(feature = "db-sqlite")]
pub type DbPool = sqlx::SqlitePool;

pub async fn connect(database_url: &str) -> anyhow::Result<DbPool> {
    #[cfg(feature = "db-sqlite")]
    {
        let _ = database_url;
        return connect_sqlite(database_url).await;
    }
    #[cfg(feature = "db-postgres")]
    {
        return connect_postgres(database_url).await;
    }
}

#[cfg(feature = "db-sqlite")]
async fn connect_sqlite(database_url: &str) -> anyhow::Result<DbPool> {
    let url = if database_url.starts_with("sqlite:") {
        database_url.to_string()
    } else {
        format!("sqlite:{database_url}")
    };

    if let Some(path) = sqlite_file_path(&url) {
        if let Some(parent) = Path::new(&path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create sqlite dir {}", parent.display()))?;
            }
        }
    }

    let options: SqliteConnectOptions = url.parse()?;
    let options = options.create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations/sqlite").run(&pool).await?;
    tracing::info!(%url, "sqlite database ready");
    Ok(pool)
}

#[cfg(feature = "db-postgres")]
async fn connect_postgres(database_url: &str) -> anyhow::Result<DbPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    sqlx::migrate!("./migrations/postgres").run(&pool).await?;
    tracing::info!("postgres database ready");
    Ok(pool)
}

#[cfg(feature = "db-sqlite")]
fn sqlite_file_path(database_url: &str) -> Option<String> {
    let rest = database_url.strip_prefix("sqlite:")?;
    let path = rest.strip_prefix("//").unwrap_or(rest);
    if path == ":memory:" || path.is_empty() {
        return None;
    }
    Some(path.to_string())
}
