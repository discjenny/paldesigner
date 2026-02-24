mod api;
mod config;
mod db;
mod save;
mod storage;

use anyhow::Context;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Clone)]
pub struct AppSettings {
    pub artifact_storage_root: PathBuf,
    pub max_import_zip_bytes: usize,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub settings: AppSettings,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    config::init_tracing();
    let cfg = config::AppConfig::from_env()?;
    let pool = db::connect(&cfg.database_url).await?;
    db::migrate(&pool).await?;

    tokio::fs::create_dir_all(&cfg.artifact_storage_root)
        .await
        .with_context(|| {
            format!(
                "failed to create artifact storage root at {}",
                cfg.artifact_storage_root.display()
            )
        })?;

    let settings = AppSettings {
        artifact_storage_root: cfg.artifact_storage_root.clone(),
        max_import_zip_bytes: cfg.max_import_zip_bytes,
    };
    let state = AppState { pool, settings };

    let app: Router = api::routes::router(state)
        .layer(DefaultBodyLimit::disable())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from((cfg.host, cfg.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind server on {}", addr))?;

    info!("server listening on http://{}", addr);
    axum::serve(listener, app).await.context("server crashed")?;
    Ok(())
}
