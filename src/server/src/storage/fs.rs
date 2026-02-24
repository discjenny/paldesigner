use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub async fn write_bytes(root: &Path, storage_key: &str, bytes: &[u8]) -> Result<PathBuf> {
    let full_path = root.join(storage_key);
    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create parent directories for {}", storage_key))?;
    }

    tokio::fs::write(&full_path, bytes)
        .await
        .with_context(|| format!("failed to write artifact bytes for {}", storage_key))?;

    Ok(full_path)
}
