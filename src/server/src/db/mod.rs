use anyhow::{Context, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

pub async fn connect(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(5))
        .max_connections(5)
        .connect(database_url)
        .await
        .with_context(|| "failed to connect to PostgreSQL")
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./src/db/migrations")
        .run(pool)
        .await
        .with_context(|| "failed to run PostgreSQL migrations")
}
