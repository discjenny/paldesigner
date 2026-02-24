use crate::AppState;
use crate::save::detect::detect_save_variant;
use crate::save::normalize::{NormalizedPlannerSummary, empty_summary};
use crate::save::parse::inspect_gvas;
use crate::save::zip::{
    detect_world_root, is_supported_world_file, parse_zip_entries, strip_root_prefix,
};
use crate::storage::fs;
use axum::Json;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;
use xxhash_rust::xxh64::xxh64;

#[derive(Serialize)]
pub struct ImportZipResponse {
    pub import_version_id: Uuid,
    pub world_root_path: String,
    pub persisted_file_count: usize,
    pub supported_file_count: usize,
    pub normalized_summary: NormalizedPlannerSummary,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

pub async fn import_zip(State(state): State<AppState>, multipart: Multipart) -> impl IntoResponse {
    match run_import_zip(&state, multipart).await {
        Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
        Err(error) => (
            error.status,
            Json(ErrorResponse {
                error: error.message,
            }),
        )
            .into_response(),
    }
}

async fn run_import_zip(
    state: &AppState,
    mut multipart: Multipart,
) -> Result<ImportZipResponse, ApiError> {
    let mut uploaded_name: Option<String> = None;
    let mut uploaded_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|error| {
        ApiError::bad_request(format!("failed to read multipart form data: {}", error))
    })? {
        let Some(file_name) = field.file_name() else {
            continue;
        };

        if uploaded_name.is_some() {
            return Err(ApiError::bad_request(
                "multipart request must contain exactly one ZIP file field",
            ));
        }

        let file_name = file_name.to_string();
        if !file_name.to_ascii_lowercase().ends_with(".zip") {
            return Err(ApiError::bad_request(
                "uploaded file must have .zip extension",
            ));
        }

        let bytes = field
            .bytes()
            .await
            .map_err(|error| {
                ApiError::bad_request(format!("failed to read uploaded ZIP bytes: {}", error))
            })?
            .to_vec();

        if bytes.len() > state.settings.max_import_zip_bytes {
            return Err(ApiError::bad_request(format!(
                "uploaded ZIP is too large ({} bytes > {} bytes)",
                bytes.len(),
                state.settings.max_import_zip_bytes
            )));
        }

        uploaded_name = Some(file_name);
        uploaded_bytes = Some(bytes);
    }

    let source_file_name = uploaded_name
        .ok_or_else(|| ApiError::bad_request("multipart request did not include a ZIP file"))?;
    let zip_bytes = uploaded_bytes
        .ok_or_else(|| ApiError::bad_request("multipart request did not include ZIP bytes"))?;

    let entries = parse_zip_entries(&zip_bytes)
        .map_err(|error| ApiError::bad_request(format!("invalid ZIP content: {}", error)))?;
    let world_root_path = detect_world_root(&entries)
        .map_err(|error| ApiError::bad_request(format!("invalid world root: {}", error)))?;

    let mut rooted_entries = BTreeMap::<String, Vec<u8>>::new();
    for entry in entries {
        if let Some(relative_path) = strip_root_prefix(&world_root_path, &entry.path) {
            if relative_path.is_empty() {
                continue;
            }

            if rooted_entries
                .insert(relative_path.clone(), entry.bytes)
                .is_some()
            {
                return Err(ApiError::bad_request(format!(
                    "ZIP contains duplicate file after root stripping: {}",
                    relative_path
                )));
            }
        }
    }

    if rooted_entries.is_empty() {
        return Err(ApiError::bad_request(
            "detected world root does not contain any files",
        ));
    }
    if rooted_entries
        .keys()
        .any(|path| path.starts_with("Player/"))
    {
        return Err(ApiError::bad_request(
            "ZIP uses Player/ directory; only Players/ is supported",
        ));
    }
    if !rooted_entries.contains_key("Level.sav") {
        return Err(ApiError::bad_request(
            "world root is missing required Level.sav",
        ));
    }
    if !rooted_entries
        .keys()
        .any(|path| path.starts_with("Players/") && path.ends_with(".sav"))
    {
        return Err(ApiError::bad_request(
            "world root is missing required Players/*.sav files",
        ));
    }

    let import_version_id = Uuid::new_v4();
    let source_zip_id = Uuid::new_v4();
    let source_zip_storage_key = format!("storage/imports/{}/source.zip", import_version_id);
    let (source_sha256, source_xxh64, source_byte_size) = compute_hashes(&zip_bytes)
        .map_err(|error| ApiError::internal(format!("failed to hash uploaded ZIP: {}", error)))?;

    fs::write_bytes(
        &state.settings.artifact_storage_root,
        &source_zip_storage_key,
        &zip_bytes,
    )
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut tx = state.pool.begin().await.map_err(|error| {
        ApiError::internal(format!("failed to start database transaction: {}", error))
    })?;

    sqlx::query(
        "INSERT INTO save_import_versions (id, source_file_name, world_root_path, status) VALUES ($1, $2, $3, 'processing')",
    )
    .bind(import_version_id)
    .bind(&source_file_name)
    .bind(&world_root_path)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal(format!("failed to insert save_import_versions row: {}", error)))?;

    sqlx::query(
        "INSERT INTO save_zip_artifacts (id, import_version_id, export_version_id, kind, storage_key, file_name, byte_size, sha256, xxh64, immutable, retention_policy)
         VALUES ($1, $2, NULL, 'import_source_zip', $3, $4, $5, $6, $7, TRUE, 'forever')",
    )
    .bind(source_zip_id)
    .bind(import_version_id)
    .bind(&source_zip_storage_key)
    .bind(&source_file_name)
    .bind(source_byte_size)
    .bind(&source_sha256)
    .bind(&source_xxh64)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal(format!("failed to insert source ZIP artifact row: {}", error)))?;

    let mut persisted_file_count = 0usize;
    let mut supported_file_count = 0usize;

    for (relative_path, bytes) in rooted_entries {
        let file_id = Uuid::new_v4();
        let storage_key = format!(
            "storage/imports/{}/files/{}",
            import_version_id, relative_path
        );
        let (sha256, xxh64, byte_size) = compute_hashes(&bytes).map_err(|error| {
            ApiError::internal(format!("failed to hash {}: {}", relative_path, error))
        })?;

        fs::write_bytes(&state.settings.artifact_storage_root, &storage_key, &bytes)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;

        let is_supported = is_supported_world_file(&relative_path);
        let ignored_reason = if is_supported {
            None
        } else {
            Some("ignored_extra_file".to_string())
        };

        sqlx::query(
            "INSERT INTO save_files (id, import_version_id, relative_path, storage_key, is_supported, ignored_reason, byte_size, sha256, xxh64, immutable, retention_policy)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE, 'forever')",
        )
        .bind(file_id)
        .bind(import_version_id)
        .bind(&relative_path)
        .bind(&storage_key)
        .bind(is_supported)
        .bind(ignored_reason)
        .bind(byte_size)
        .bind(&sha256)
        .bind(&xxh64)
        .execute(&mut *tx)
        .await
        .map_err(|error| ApiError::internal(format!("failed to insert save_files row for {}: {}", relative_path, error)))?;

        persisted_file_count += 1;
        if is_supported {
            supported_file_count += 1;
        }

        if is_supported && relative_path.ends_with(".sav") {
            let variant = detect_save_variant(&bytes);
            let gvas = inspect_gvas(&bytes, &variant);

            sqlx::query(
                "INSERT INTO save_variant_metadata (id, save_file_id, has_cnk_prefix, magic, save_type, compression, uncompressed_size, compressed_size, gvas_magic, decompressed_size, decode_status, decode_error)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            )
            .bind(Uuid::new_v4())
            .bind(file_id)
            .bind(variant.has_cnk_prefix)
            .bind(variant.magic)
            .bind(variant.save_type.map(i16::from))
            .bind(variant.compression)
            .bind(variant.uncompressed_size.map(i64::from))
            .bind(variant.compressed_size.map(i64::from))
            .bind(gvas.gvas_magic)
            .bind(gvas.decompressed_size.map(|size| size as i64))
            .bind(gvas.decode_status)
            .bind(gvas.decode_error)
            .execute(&mut *tx)
            .await
            .map_err(|error| ApiError::internal(format!(
                "failed to insert save_variant_metadata row for {}: {}",
                relative_path, error
            )))?;
        }
    }

    sqlx::query(
        "UPDATE save_import_versions SET status = 'ready', completed_at = NOW() WHERE id = $1",
    )
    .bind(import_version_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| {
        ApiError::internal(format!(
            "failed to finalize save_import_versions row: {}",
            error
        ))
    })?;

    tx.commit().await.map_err(|error| {
        ApiError::internal(format!("failed to commit import transaction: {}", error))
    })?;

    Ok(ImportZipResponse {
        import_version_id,
        world_root_path,
        persisted_file_count,
        supported_file_count,
        normalized_summary: empty_summary(),
    })
}

fn compute_hashes(bytes: &[u8]) -> Result<(String, String, i64), std::num::TryFromIntError> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let sha256 = format!("{:x}", hasher.finalize());

    let xxh64_hex = format!("{:016x}", xxh64(bytes, 0));
    let byte_size = i64::try_from(bytes.len())?;

    Ok((sha256, xxh64_hex, byte_size))
}
