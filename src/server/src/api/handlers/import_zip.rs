use crate::AppState;
use crate::save::detect::detect_save_variant;
use crate::save::normalize::{
    self, ExtractedAssignment, ExtractedPal, ExtractedPlayer, NormalizedPlannerSummary,
};
use crate::save::parse::inspect_gvas;
use crate::save::zip::{
    detect_world_root, is_supported_world_file, parse_zip_entries, strip_root_prefix,
};
use crate::storage::fs;
use anyhow::Context;
use axum::Json;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use tracing::{error, warn};
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
        "INSERT INTO save_import_versions (
            id, source_file_name, world_root_path, status, progress_phase, progress_pct, progress_message
         ) VALUES ($1, $2, $3, 'processing', 'persisting_artifacts', 10, 'Persisting ZIP and extracted files')",
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
    let mut normalized_player_seed_rows: Vec<(String, Uuid, String)> = Vec::new();

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

        if let Some(player_uid) = extract_player_uid_from_path(&relative_path) {
            normalized_player_seed_rows.push((player_uid, file_id, relative_path.clone()));
        }
    }

    let normalized_summary;

    for (player_uid, raw_file_ref, raw_entity_path) in normalized_player_seed_rows {
        let planner_player_id = upsert_planner_player(
            &mut tx,
            import_version_id,
            &ExtractedPlayer {
                player_uid,
                player_instance_id: None,
                player_name: None,
                guild_id: None,
                level: None,
                raw_file_ref,
                raw_entity_path: raw_entity_path.clone(),
            },
        )
        .await
        .map_err(|error| {
            ApiError::internal(format!(
                "failed to seed planner_players row from Players/*.sav path: {}",
                error
            ))
        })?;

        sqlx::query(
            "INSERT INTO planner_player_links (id, planner_player_id, save_file_id, raw_entity_path)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (planner_player_id, save_file_id, raw_entity_path) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(planner_player_id)
        .bind(raw_file_ref)
        .bind(&raw_entity_path)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            ApiError::internal(format!(
                "failed to seed planner_player_links row from Players/*.sav path: {}",
                error
            ))
        })?;
    }

    let summary_row = sqlx::query(
        "SELECT
            (SELECT COUNT(*) FROM planner_players pp WHERE pp.import_version_id = $1) AS player_count,
            (SELECT COUNT(*) FROM planner_pals pp WHERE pp.import_version_id = $1) AS pal_count,
            (SELECT COUNT(*) FROM planner_base_assignments pba WHERE pba.import_version_id = $1) AS base_assignment_count",
    )
    .bind(import_version_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| {
        ApiError::internal(format!(
            "failed to compute normalized summary counts: {}",
            error
        ))
    })?;
    normalized_summary = NormalizedPlannerSummary {
        player_count: usize::try_from(summary_row.get::<i64, _>("player_count")).unwrap_or(0),
        pal_count: usize::try_from(summary_row.get::<i64, _>("pal_count")).unwrap_or(0),
        base_assignment_count: usize::try_from(summary_row.get::<i64, _>("base_assignment_count"))
            .unwrap_or(0),
    };

    sqlx::query(
        "UPDATE save_import_versions
         SET status = 'processing',
             progress_phase = 'queued_decode',
             progress_pct = 35,
             progress_message = 'Queued decode and normalization',
             failed_error = NULL
         WHERE id = $1",
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

    spawn_post_import_processing(state.clone(), import_version_id);

    Ok(ImportZipResponse {
        import_version_id,
        world_root_path,
        persisted_file_count,
        supported_file_count,
        normalized_summary,
    })
}

fn spawn_post_import_processing(state: AppState, import_version_id: Uuid) {
    tokio::spawn(async move {
        if let Err(error) = run_post_import_processing(state.clone(), import_version_id).await {
            error!(
                import_version_id = %import_version_id,
                "post-import processing failed: {error:#}"
            );
            if let Err(mark_error) =
                mark_import_failed(&state, import_version_id, &error.to_string()).await
            {
                error!(
                    import_version_id = %import_version_id,
                    "failed to mark import as failed after background error: {mark_error:#}"
                );
            }
        }
    });
}

async fn run_post_import_processing(
    state: AppState,
    import_version_id: Uuid,
) -> anyhow::Result<()> {
    const VARIANT_INSPECT_TIMEOUT_SECS: u64 = 20;
    const LEVEL_NORMALIZE_TIMEOUT_SECS: u64 = 300;

    update_import_progress(
        &state,
        import_version_id,
        "decoding_variants",
        50,
        "Decoding SAV wrappers and GVAS headers",
    )
    .await?;

    let save_files = sqlx::query(
        "SELECT id, relative_path, storage_key, is_supported
         FROM save_files
         WHERE import_version_id = $1
         ORDER BY relative_path ASC",
    )
    .bind(import_version_id)
    .fetch_all(&state.pool)
    .await
    .with_context(|| format!("failed to load save_files for import {}", import_version_id))?;

    let mut level_sav_for_normalize: Option<(Uuid, Vec<u8>)> = None;
    let mut parse_metrics_json: Option<Value> = None;

    for row in save_files {
        let save_file_id: Uuid = row.get("id");
        let relative_path: String = row.get("relative_path");
        let storage_key: String = row.get("storage_key");
        let is_supported: bool = row.get("is_supported");

        if !is_supported || !relative_path.ends_with(".sav") {
            continue;
        }

        let bytes = fs::read_bytes(&state.settings.artifact_storage_root, &storage_key)
            .await
            .with_context(|| format!("failed to load bytes for {}", relative_path))?;

        let bytes_for_inspect = bytes.clone();
        let inspect_result = tokio::time::timeout(
            Duration::from_secs(VARIANT_INSPECT_TIMEOUT_SECS),
            tokio::task::spawn_blocking(move || {
                let variant = detect_save_variant(&bytes_for_inspect);
                let gvas = inspect_gvas(&bytes_for_inspect, &variant);
                (variant, gvas)
            }),
        )
        .await;

        let (variant, gvas) = match inspect_result {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => {
                return Err(anyhow::anyhow!(
                    "variant inspect worker panicked for {}: {}",
                    relative_path,
                    error
                ));
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "variant inspect timed out after {}s for {}",
                    VARIANT_INSPECT_TIMEOUT_SECS,
                    relative_path
                ));
            }
        };

        sqlx::query(
            "INSERT INTO save_variant_metadata (
                id, save_file_id, has_cnk_prefix, magic, save_type, compression, uncompressed_size, compressed_size,
                gvas_magic, decompressed_size, decode_status, decode_error
             ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                $9, $10, $11, $12
             )
             ON CONFLICT (save_file_id) DO UPDATE SET
                has_cnk_prefix = EXCLUDED.has_cnk_prefix,
                magic = EXCLUDED.magic,
                save_type = EXCLUDED.save_type,
                compression = EXCLUDED.compression,
                uncompressed_size = EXCLUDED.uncompressed_size,
                compressed_size = EXCLUDED.compressed_size,
                gvas_magic = EXCLUDED.gvas_magic,
                decompressed_size = EXCLUDED.decompressed_size,
                decode_status = EXCLUDED.decode_status,
                decode_error = EXCLUDED.decode_error",
        )
        .bind(Uuid::new_v4())
        .bind(save_file_id)
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
        .execute(&state.pool)
        .await
        .with_context(|| format!("failed to upsert save_variant_metadata for {}", relative_path))?;

        if relative_path == "Level.sav" {
            level_sav_for_normalize = Some((save_file_id, bytes));
        }
    }

    update_import_progress(
        &state,
        import_version_id,
        "normalizing_entities",
        75,
        "Extracting planner entities from Level.sav",
    )
    .await?;

    if let Some((level_file_id, level_bytes)) = level_sav_for_normalize {
        let (progress_tx, mut progress_rx) =
            tokio::sync::mpsc::unbounded_channel::<normalize::NormalizationProgress>();
        let mut normalize_worker = tokio::task::spawn_blocking(move || {
            normalize::extract_from_level_sav_with_progress(
                &level_bytes,
                level_file_id,
                |progress| {
                    let _ = progress_tx.send(progress);
                },
            )
        });
        let timeout = tokio::time::sleep(Duration::from_secs(LEVEL_NORMALIZE_TIMEOUT_SECS));
        tokio::pin!(timeout);

        let mut last_processed_report: Option<usize> = None;
        let mut last_progress_pct: Option<i32> = None;
        let mut last_progress_message: Option<String> = None;
        let mut last_reported_at = Instant::now() - Duration::from_secs(10);
        let extracted = loop {
            tokio::select! {
                maybe_progress = progress_rx.recv() => {
                    let Some(progress) = maybe_progress else {
                        continue;
                    };

                    let force_report = progress.total_character_entries > 0
                        && progress.processed_character_entries == progress.total_character_entries;
                    let progress_pct = progress.progress_pct_hint.unwrap_or(75).clamp(75, 98);
                    let should_report = force_report
                        || last_processed_report.is_none()
                        || last_progress_pct != Some(progress_pct)
                        || progress.processed_character_entries.saturating_sub(last_processed_report.unwrap_or_default()) >= 256
                        || (last_reported_at.elapsed() >= Duration::from_millis(900)
                            && last_progress_message.as_deref() != Some(progress.message.as_str()));

                    if should_report {
                        update_import_progress(
                            &state,
                            import_version_id,
                            "normalizing_entities",
                            progress_pct,
                            &progress.message,
                        ).await?;
                        last_processed_report = Some(progress.processed_character_entries);
                        last_progress_pct = Some(progress_pct);
                        last_progress_message = Some(progress.message);
                        last_reported_at = Instant::now();
                    }
                }
                joined = &mut normalize_worker => {
                    match joined {
                        Ok(Ok(extracted)) => break Some(extracted),
                        Ok(Err(error)) => {
                            warn!(
                                import_version_id = %import_version_id,
                                "level normalization skipped: {}",
                                error
                            );
                            break None;
                        }
                        Err(error) => {
                            return Err(anyhow::anyhow!(
                                "level normalization worker panicked for {}: {}",
                                import_version_id,
                                error
                            ));
                        }
                    }
                }
                _ = &mut timeout => {
                    normalize_worker.abort();
                    return Err(anyhow::anyhow!(
                        "level normalization timed out after {}s for {}",
                        LEVEL_NORMALIZE_TIMEOUT_SECS,
                        import_version_id
                    ));
                }
            }
        };

        if let Some(normalized) = extracted {
            parse_metrics_json = Some(serde_json::to_value(&normalized.metrics)?);
            persist_normalized_extract(&state, import_version_id, normalized.data).await?;
        }
    }

    sqlx::query(
        "UPDATE save_import_versions
         SET status = 'ready',
             progress_phase = 'complete',
             progress_pct = 100,
             progress_message = 'Import processing complete',
             parse_metrics_json = $2,
             failed_error = NULL,
             completed_at = NOW()
         WHERE id = $1",
    )
    .bind(import_version_id)
    .bind(parse_metrics_json)
    .execute(&state.pool)
    .await
    .with_context(|| format!("failed to finalize import {}", import_version_id))?;

    Ok(())
}

async fn persist_normalized_extract(
    state: &AppState,
    import_version_id: Uuid,
    extracted: normalize::ExtractedPlannerData,
) -> anyhow::Result<()> {
    let mut tx = state.pool.begin().await.with_context(|| {
        format!(
            "failed to open normalize transaction for {}",
            import_version_id
        )
    })?;

    for player in extracted.players {
        let planner_player_id = upsert_planner_player(&mut tx, import_version_id, &player)
            .await
            .with_context(|| "failed to upsert normalized player row")?;

        sqlx::query(
            "INSERT INTO planner_player_links (id, planner_player_id, save_file_id, raw_entity_path)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (planner_player_id, save_file_id, raw_entity_path) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(planner_player_id)
        .bind(player.raw_file_ref)
        .bind(&player.raw_entity_path)
        .execute(&mut *tx)
        .await
        .with_context(|| "failed to upsert normalized player link row")?;
    }

    for pal in extracted.pals {
        let planner_pal_id = upsert_planner_pal(&mut tx, import_version_id, &pal)
            .await
            .with_context(|| "failed to upsert normalized pal row")?;

        sqlx::query(
            "INSERT INTO planner_pal_links (id, planner_pal_id, save_file_id, raw_entity_path)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (planner_pal_id, save_file_id, raw_entity_path) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(planner_pal_id)
        .bind(pal.raw_file_ref)
        .bind(&pal.raw_entity_path)
        .execute(&mut *tx)
        .await
        .with_context(|| "failed to upsert normalized pal link row")?;
    }

    for assignment in extracted.assignments {
        let planner_assignment_id =
            upsert_planner_assignment(&mut tx, import_version_id, &assignment)
                .await
                .with_context(|| "failed to upsert normalized assignment row")?;

        sqlx::query(
            "INSERT INTO planner_base_assignment_links (id, planner_base_assignment_id, save_file_id, raw_entity_path)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (planner_base_assignment_id, save_file_id, raw_entity_path) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(planner_assignment_id)
        .bind(assignment.raw_file_ref)
        .bind(&assignment.raw_entity_path)
        .execute(&mut *tx)
        .await
        .with_context(|| "failed to upsert normalized assignment link row")?;
    }

    tx.commit().await.with_context(|| {
        format!(
            "failed to commit normalized extract for {}",
            import_version_id
        )
    })?;

    Ok(())
}

async fn update_import_progress(
    state: &AppState,
    import_version_id: Uuid,
    phase: &str,
    progress_pct: i32,
    message: &str,
) -> anyhow::Result<()> {
    let clamped_pct = progress_pct.clamp(0, 100);
    sqlx::query(
        "UPDATE save_import_versions
         SET status = 'processing',
             progress_phase = $2,
             progress_pct = $3,
             progress_message = $4,
             failed_error = NULL
         WHERE id = $1",
    )
    .bind(import_version_id)
    .bind(phase)
    .bind(clamped_pct)
    .bind(message)
    .execute(&state.pool)
    .await
    .with_context(|| format!("failed to update import progress for {}", import_version_id))?;
    Ok(())
}

async fn mark_import_failed(
    state: &AppState,
    import_version_id: Uuid,
    error_text: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE save_import_versions
         SET status = 'failed',
             progress_phase = 'failed',
             progress_pct = 100,
             progress_message = 'Import processing failed',
             failed_error = $2,
             completed_at = NOW()
         WHERE id = $1",
    )
    .bind(import_version_id)
    .bind(error_text)
    .execute(&state.pool)
    .await
    .with_context(|| format!("failed to mark import {} as failed", import_version_id))?;
    Ok(())
}

fn compute_hashes(bytes: &[u8]) -> Result<(String, String, i64), std::num::TryFromIntError> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let sha256 = format!("{:x}", hasher.finalize());

    let xxh64_hex = format!("{:016x}", xxh64(bytes, 0));
    let byte_size = i64::try_from(bytes.len())?;

    Ok((sha256, xxh64_hex, byte_size))
}

fn extract_player_uid_from_path(relative_path: &str) -> Option<String> {
    let file_name = relative_path
        .strip_prefix("Players/")?
        .strip_suffix(".sav")?;

    if file_name.len() == 32 && file_name.chars().all(|value| value.is_ascii_hexdigit()) {
        Some(file_name.to_uppercase())
    } else {
        None
    }
}

async fn upsert_planner_player(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    import_version_id: Uuid,
    player: &ExtractedPlayer,
) -> Result<Uuid, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO planner_players (id, import_version_id, player_uid, player_instance_id, player_name, guild_id, level, raw_file_ref, raw_entity_path)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (import_version_id, player_uid) DO UPDATE SET
            player_instance_id = COALESCE(EXCLUDED.player_instance_id, planner_players.player_instance_id),
            player_name = COALESCE(EXCLUDED.player_name, planner_players.player_name),
            guild_id = COALESCE(EXCLUDED.guild_id, planner_players.guild_id),
            level = COALESCE(EXCLUDED.level, planner_players.level),
            raw_file_ref = EXCLUDED.raw_file_ref,
            raw_entity_path = EXCLUDED.raw_entity_path
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(import_version_id)
    .bind(&player.player_uid)
    .bind(&player.player_instance_id)
    .bind(&player.player_name)
    .bind(&player.guild_id)
    .bind(player.level)
    .bind(player.raw_file_ref)
    .bind(&player.raw_entity_path)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.get("id"))
}

async fn upsert_planner_pal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    import_version_id: Uuid,
    pal: &ExtractedPal,
) -> Result<Uuid, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO planner_pals (
            id, import_version_id, pal_instance_id, owner_player_uid, species_id, nickname, gender, level, exp,
            passive_skill_ids, mastered_waza_ids, equip_waza_ids, work_suitability_ranks, raw_file_ref, raw_entity_path
         ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9,
            $10, $11, $12, $13, $14, $15
         )
         ON CONFLICT (import_version_id, pal_instance_id) DO UPDATE SET
            owner_player_uid = COALESCE(EXCLUDED.owner_player_uid, planner_pals.owner_player_uid),
            species_id = COALESCE(EXCLUDED.species_id, planner_pals.species_id),
            nickname = COALESCE(EXCLUDED.nickname, planner_pals.nickname),
            gender = COALESCE(EXCLUDED.gender, planner_pals.gender),
            level = COALESCE(EXCLUDED.level, planner_pals.level),
            exp = COALESCE(EXCLUDED.exp, planner_pals.exp),
            passive_skill_ids = EXCLUDED.passive_skill_ids,
            mastered_waza_ids = EXCLUDED.mastered_waza_ids,
            equip_waza_ids = EXCLUDED.equip_waza_ids,
            work_suitability_ranks = EXCLUDED.work_suitability_ranks,
            raw_file_ref = EXCLUDED.raw_file_ref,
            raw_entity_path = EXCLUDED.raw_entity_path
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(import_version_id)
    .bind(&pal.pal_instance_id)
    .bind(&pal.owner_player_uid)
    .bind(&pal.species_id)
    .bind(&pal.nickname)
    .bind(&pal.gender)
    .bind(pal.level)
    .bind(pal.exp)
    .bind(pal.passive_skill_ids_json())
    .bind(pal.mastered_waza_ids_json())
    .bind(pal.equip_waza_ids_json())
    .bind(json!({}))
    .bind(pal.raw_file_ref)
    .bind(&pal.raw_entity_path)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.get("id"))
}

async fn upsert_planner_assignment(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    import_version_id: Uuid,
    assignment: &ExtractedAssignment,
) -> Result<Uuid, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO planner_base_assignments (
            id, import_version_id, base_id, pal_instance_id, assignment_kind, assignment_target, priority, raw_file_ref, raw_entity_path
         ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9
         )
         ON CONFLICT (import_version_id, base_id, pal_instance_id, assignment_kind, assignment_target) DO UPDATE SET
            priority = EXCLUDED.priority,
            raw_file_ref = EXCLUDED.raw_file_ref,
            raw_entity_path = EXCLUDED.raw_entity_path
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(import_version_id)
    .bind(&assignment.base_id)
    .bind(&assignment.pal_instance_id)
    .bind(&assignment.assignment_kind)
    .bind(&assignment.assignment_target)
    .bind(assignment.priority)
    .bind(assignment.raw_file_ref)
    .bind(&assignment.raw_entity_path)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.get("id"))
}
