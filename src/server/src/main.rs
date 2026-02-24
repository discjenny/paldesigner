mod api;
mod config;
mod db;

use anyhow::Context;
use axum::Router;
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    config::init_tracing();
    let cfg = config::AppConfig::from_env()?;
    let pool = db::connect(&cfg.database_url).await?;
    let state = AppState { pool };

    let app: Router = api::routes::router(state)
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
