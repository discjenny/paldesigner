use anyhow::{Context, Result};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: Ipv4Addr,
    pub port: u16,
    pub database_url: String,
    pub artifact_storage_root: PathBuf,
    pub max_import_zip_bytes: usize,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let host = std::env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("APP_PORT").unwrap_or_else(|_| "8080".to_string());
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL is required (example: postgres://postgres:postgres@localhost:5432/paldesigner)")?;
        let artifact_storage_root =
            std::env::var("ARTIFACT_STORAGE_ROOT").unwrap_or_else(|_| ".".to_string());
        let max_import_zip_bytes = std::env::var("MAX_IMPORT_ZIP_BYTES")
            .unwrap_or_else(|_| "157286400".to_string())
            .parse()
            .context("MAX_IMPORT_ZIP_BYTES must be a valid usize")?;

        Ok(Self {
            host: host
                .parse()
                .context("APP_HOST must be a valid IPv4 address")?,
            port: port.parse().context("APP_PORT must be a valid u16")?,
            database_url,
            artifact_storage_root: PathBuf::from(artifact_storage_root),
            max_import_zip_bytes,
        })
    }
}

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let use_json = std::env::var("LOG_JSON")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if use_json {
        fmt().with_env_filter(filter).json().init();
    } else {
        fmt().with_env_filter(filter).init();
    }
}
