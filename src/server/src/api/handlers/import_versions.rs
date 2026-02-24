use crate::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use serde::Serialize;
use serde_json::Value;
use sqlx::Row;
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ImportVersionListItem {
    pub id: Uuid,
    pub source_file_name: String,
    pub world_root_path: String,
    pub status: String,
    pub progress_phase: String,
    pub progress_pct: i32,
    pub progress_message: String,
    pub failed_error: Option<String>,
    pub parse_metrics_json: Option<Value>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub file_count: i64,
    pub supported_file_count: i64,
    pub variant_row_count: i64,
    pub player_count: i64,
    pub pal_count: i64,
    pub base_assignment_count: i64,
}

#[derive(Serialize)]
pub struct ImportVersionListResponse {
    pub versions: Vec<ImportVersionListItem>,
}

#[derive(Serialize)]
pub struct ImportVersionFileItem {
    pub id: Uuid,
    pub relative_path: String,
    pub is_supported: bool,
    pub ignored_reason: Option<String>,
    pub byte_size: i64,
    pub sha256: String,
    pub xxh64: String,
    pub created_at: String,
    pub has_cnk_prefix: Option<bool>,
    pub magic: Option<String>,
    pub save_type: Option<i16>,
    pub compression: Option<String>,
    pub uncompressed_size: Option<i64>,
    pub compressed_size: Option<i64>,
    pub gvas_magic: Option<String>,
    pub decompressed_size: Option<i64>,
    pub decode_status: Option<String>,
    pub decode_error: Option<String>,
}

#[derive(Serialize)]
pub struct ImportVersionDetailResponse {
    pub version: ImportVersionListItem,
    pub files: Vec<ImportVersionFileItem>,
}

#[derive(Serialize)]
pub struct NormalizedPlayerRow {
    pub id: Uuid,
    pub player_uid: String,
    pub player_instance_id: Option<String>,
    pub player_name: Option<String>,
    pub guild_id: Option<String>,
    pub level: Option<i32>,
    pub raw_file_ref: Option<Uuid>,
    pub raw_entity_path: String,
}

#[derive(Serialize)]
pub struct NormalizedPalRow {
    pub id: Uuid,
    pub pal_instance_id: String,
    pub owner_player_uid: Option<String>,
    pub species_id: Option<String>,
    pub nickname: Option<String>,
    pub level: Option<i32>,
    pub raw_file_ref: Option<Uuid>,
    pub raw_entity_path: String,
}

#[derive(Serialize)]
pub struct NormalizedAssignmentRow {
    pub id: Uuid,
    pub base_id: String,
    pub pal_instance_id: String,
    pub assignment_kind: Option<String>,
    pub assignment_target: Option<String>,
    pub priority: Option<i32>,
    pub raw_file_ref: Option<Uuid>,
    pub raw_entity_path: String,
}

#[derive(Serialize)]
pub struct NormalizedResponse {
    pub import_version_id: Uuid,
    pub players: Vec<NormalizedPlayerRow>,
    pub pals: Vec<NormalizedPalRow>,
    pub base_assignments: Vec<NormalizedAssignmentRow>,
}

#[derive(Serialize, Clone, PartialEq)]
pub struct ImportProgressEvent {
    pub import_version_id: Uuid,
    pub status: String,
    pub progress_phase: String,
    pub progress_pct: i32,
    pub progress_message: String,
    pub failed_error: Option<String>,
    pub parse_metrics_json: Option<Value>,
    pub completed_at: Option<String>,
    pub player_count: i64,
    pub pal_count: i64,
    pub base_assignment_count: i64,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

pub async fn list_import_versions(State(state): State<AppState>) -> impl IntoResponse {
    match run_list_import_versions(&state).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("failed to list import versions: {}", error),
            }),
        )
            .into_response(),
    }
}

pub async fn get_import_version(
    State(state): State<AppState>,
    Path(import_version_id): Path<Uuid>,
) -> impl IntoResponse {
    match run_get_import_version(&state, import_version_id).await {
        Ok(Some(response)) => (StatusCode::OK, Json(response)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "import version not found".to_string(),
            }),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("failed to get import version: {}", error),
            }),
        )
            .into_response(),
    }
}

pub async fn get_normalized(
    State(state): State<AppState>,
    Path(import_version_id): Path<Uuid>,
) -> impl IntoResponse {
    match run_get_normalized(&state, import_version_id).await {
        Ok(Some(response)) => (StatusCode::OK, Json(response)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "import version not found".to_string(),
            }),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("failed to get normalized rows: {}", error),
            }),
        )
            .into_response(),
    }
}

pub async fn stream_import_version_events(
    State(state): State<AppState>,
    Path(import_version_id): Path<Uuid>,
) -> impl IntoResponse {
    let stream = async_stream::stream! {
        let mut last_payload: Option<ImportProgressEvent> = None;
        loop {
            match fetch_import_progress(&state, import_version_id).await {
                Ok(Some(progress)) => {
                    if last_payload.as_ref() != Some(&progress) {
                        let data = match serde_json::to_string(&progress) {
                            Ok(value) => value,
                            Err(error) => {
                                let fallback = serde_json::json!({
                                    "error": format!("failed to serialize progress event: {}", error)
                                }).to_string();
                                yield Ok::<Event, Infallible>(
                                    Event::default().event("progress_error").data(fallback),
                                );
                                break;
                            }
                        };
                        yield Ok::<Event, Infallible>(Event::default().event("progress").data(data));
                        last_payload = Some(progress.clone());
                    }

                    if progress.status == "ready" || progress.status == "failed" {
                        yield Ok::<Event, Infallible>(
                            Event::default().event("done").data("{\"done\":true}"),
                        );
                        break;
                    }
                }
                Ok(None) => {
                    let data = serde_json::json!({"error":"import version not found"}).to_string();
                    yield Ok::<Event, Infallible>(
                        Event::default().event("progress_error").data(data),
                    );
                    break;
                }
                Err(error) => {
                    let data = serde_json::json!({"error": format!("failed to load progress: {}", error)}).to_string();
                    yield Ok::<Event, Infallible>(
                        Event::default().event("progress_error").data(data),
                    );
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(700)).await;
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive"),
    )
}

async fn run_list_import_versions(
    state: &AppState,
) -> Result<ImportVersionListResponse, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT
            iv.id,
            iv.source_file_name,
            iv.world_root_path,
            iv.status,
            iv.progress_phase,
            iv.progress_pct,
            iv.progress_message,
            iv.failed_error,
            iv.parse_metrics_json,
            iv.created_at::text AS created_at,
            iv.completed_at::text AS completed_at,
            (SELECT COUNT(*) FROM save_files sf WHERE sf.import_version_id = iv.id) AS file_count,
            (SELECT COUNT(*) FROM save_files sf WHERE sf.import_version_id = iv.id AND sf.is_supported) AS supported_file_count,
            (SELECT COUNT(*) FROM save_variant_metadata vm JOIN save_files sf ON sf.id = vm.save_file_id WHERE sf.import_version_id = iv.id) AS variant_row_count,
            (SELECT COUNT(*) FROM planner_players pp WHERE pp.import_version_id = iv.id) AS player_count,
            (SELECT COUNT(*) FROM planner_pals pp WHERE pp.import_version_id = iv.id) AS pal_count,
            (SELECT COUNT(*) FROM planner_base_assignments pba WHERE pba.import_version_id = iv.id) AS base_assignment_count
         FROM save_import_versions iv
         ORDER BY iv.created_at DESC
         LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await?;

    let mut versions = Vec::with_capacity(rows.len());
    for row in rows {
        versions.push(ImportVersionListItem {
            id: row.get("id"),
            source_file_name: row.get("source_file_name"),
            world_root_path: row.get("world_root_path"),
            status: row.get("status"),
            progress_phase: row.get("progress_phase"),
            progress_pct: row.get("progress_pct"),
            progress_message: row.get("progress_message"),
            failed_error: row.get("failed_error"),
            parse_metrics_json: row.get("parse_metrics_json"),
            created_at: row.get("created_at"),
            completed_at: row.get("completed_at"),
            file_count: row.get("file_count"),
            supported_file_count: row.get("supported_file_count"),
            variant_row_count: row.get("variant_row_count"),
            player_count: row.get("player_count"),
            pal_count: row.get("pal_count"),
            base_assignment_count: row.get("base_assignment_count"),
        });
    }

    Ok(ImportVersionListResponse { versions })
}

async fn run_get_import_version(
    state: &AppState,
    import_version_id: Uuid,
) -> Result<Option<ImportVersionDetailResponse>, sqlx::Error> {
    let version_rows = sqlx::query(
        "SELECT
            iv.id,
            iv.source_file_name,
            iv.world_root_path,
            iv.status,
            iv.progress_phase,
            iv.progress_pct,
            iv.progress_message,
            iv.failed_error,
            iv.parse_metrics_json,
            iv.created_at::text AS created_at,
            iv.completed_at::text AS completed_at,
            (SELECT COUNT(*) FROM save_files sf WHERE sf.import_version_id = iv.id) AS file_count,
            (SELECT COUNT(*) FROM save_files sf WHERE sf.import_version_id = iv.id AND sf.is_supported) AS supported_file_count,
            (SELECT COUNT(*) FROM save_variant_metadata vm JOIN save_files sf ON sf.id = vm.save_file_id WHERE sf.import_version_id = iv.id) AS variant_row_count,
            (SELECT COUNT(*) FROM planner_players pp WHERE pp.import_version_id = iv.id) AS player_count,
            (SELECT COUNT(*) FROM planner_pals pp WHERE pp.import_version_id = iv.id) AS pal_count,
            (SELECT COUNT(*) FROM planner_base_assignments pba WHERE pba.import_version_id = iv.id) AS base_assignment_count
         FROM save_import_versions iv
         WHERE iv.id = $1",
    )
    .bind(import_version_id)
    .fetch_all(&state.pool)
    .await?;

    if version_rows.is_empty() {
        return Ok(None);
    }

    let row = &version_rows[0];
    let version = ImportVersionListItem {
        id: row.get("id"),
        source_file_name: row.get("source_file_name"),
        world_root_path: row.get("world_root_path"),
        status: row.get("status"),
        progress_phase: row.get("progress_phase"),
        progress_pct: row.get("progress_pct"),
        progress_message: row.get("progress_message"),
        failed_error: row.get("failed_error"),
        parse_metrics_json: row.get("parse_metrics_json"),
        created_at: row.get("created_at"),
        completed_at: row.get("completed_at"),
        file_count: row.get("file_count"),
        supported_file_count: row.get("supported_file_count"),
        variant_row_count: row.get("variant_row_count"),
        player_count: row.get("player_count"),
        pal_count: row.get("pal_count"),
        base_assignment_count: row.get("base_assignment_count"),
    };

    let file_rows = sqlx::query(
        "SELECT
            sf.id,
            sf.relative_path,
            sf.is_supported,
            sf.ignored_reason,
            sf.byte_size,
            sf.sha256,
            sf.xxh64,
            sf.created_at::text AS created_at,
            vm.has_cnk_prefix,
            vm.magic,
            vm.save_type,
            vm.compression,
            vm.uncompressed_size,
            vm.compressed_size,
            vm.gvas_magic,
            vm.decompressed_size,
            vm.decode_status,
            vm.decode_error
         FROM save_files sf
         LEFT JOIN save_variant_metadata vm ON vm.save_file_id = sf.id
         WHERE sf.import_version_id = $1
         ORDER BY sf.relative_path ASC",
    )
    .bind(import_version_id)
    .fetch_all(&state.pool)
    .await?;

    let mut files = Vec::with_capacity(file_rows.len());
    for row in file_rows {
        files.push(ImportVersionFileItem {
            id: row.get("id"),
            relative_path: row.get("relative_path"),
            is_supported: row.get("is_supported"),
            ignored_reason: row.get("ignored_reason"),
            byte_size: row.get("byte_size"),
            sha256: row.get("sha256"),
            xxh64: row.get("xxh64"),
            created_at: row.get("created_at"),
            has_cnk_prefix: row.get("has_cnk_prefix"),
            magic: row.get("magic"),
            save_type: row.get("save_type"),
            compression: row.get("compression"),
            uncompressed_size: row.get("uncompressed_size"),
            compressed_size: row.get("compressed_size"),
            gvas_magic: row.get("gvas_magic"),
            decompressed_size: row.get("decompressed_size"),
            decode_status: row.get("decode_status"),
            decode_error: row.get("decode_error"),
        });
    }

    Ok(Some(ImportVersionDetailResponse { version, files }))
}

async fn run_get_normalized(
    state: &AppState,
    import_version_id: Uuid,
) -> Result<Option<NormalizedResponse>, sqlx::Error> {
    let exists = sqlx::query("SELECT id FROM save_import_versions WHERE id = $1")
        .bind(import_version_id)
        .fetch_optional(&state.pool)
        .await?;

    if exists.is_none() {
        return Ok(None);
    }

    let player_rows = sqlx::query(
        "SELECT id, player_uid, player_instance_id, player_name, guild_id, level, raw_file_ref, raw_entity_path
         FROM planner_players
         WHERE import_version_id = $1
         ORDER BY player_uid ASC",
    )
    .bind(import_version_id)
    .fetch_all(&state.pool)
    .await?;

    let pal_rows = sqlx::query(
        "SELECT id, pal_instance_id, owner_player_uid, species_id, nickname, level, raw_file_ref, raw_entity_path
         FROM planner_pals
         WHERE import_version_id = $1
         ORDER BY pal_instance_id ASC",
    )
    .bind(import_version_id)
    .fetch_all(&state.pool)
    .await?;

    let assignment_rows = sqlx::query(
        "SELECT id, base_id, pal_instance_id, assignment_kind, assignment_target, priority, raw_file_ref, raw_entity_path
         FROM planner_base_assignments
         WHERE import_version_id = $1
         ORDER BY base_id ASC, pal_instance_id ASC",
    )
    .bind(import_version_id)
    .fetch_all(&state.pool)
    .await?;

    let mut players = Vec::with_capacity(player_rows.len());
    for row in player_rows {
        players.push(NormalizedPlayerRow {
            id: row.get("id"),
            player_uid: row.get("player_uid"),
            player_instance_id: row.get("player_instance_id"),
            player_name: row.get("player_name"),
            guild_id: row.get("guild_id"),
            level: row.get("level"),
            raw_file_ref: row.get("raw_file_ref"),
            raw_entity_path: row.get("raw_entity_path"),
        });
    }

    let mut pals = Vec::with_capacity(pal_rows.len());
    for row in pal_rows {
        pals.push(NormalizedPalRow {
            id: row.get("id"),
            pal_instance_id: row.get("pal_instance_id"),
            owner_player_uid: row.get("owner_player_uid"),
            species_id: row.get("species_id"),
            nickname: row.get("nickname"),
            level: row.get("level"),
            raw_file_ref: row.get("raw_file_ref"),
            raw_entity_path: row.get("raw_entity_path"),
        });
    }

    let mut base_assignments = Vec::with_capacity(assignment_rows.len());
    for row in assignment_rows {
        base_assignments.push(NormalizedAssignmentRow {
            id: row.get("id"),
            base_id: row.get("base_id"),
            pal_instance_id: row.get("pal_instance_id"),
            assignment_kind: row.get("assignment_kind"),
            assignment_target: row.get("assignment_target"),
            priority: row.get("priority"),
            raw_file_ref: row.get("raw_file_ref"),
            raw_entity_path: row.get("raw_entity_path"),
        });
    }

    Ok(Some(NormalizedResponse {
        import_version_id,
        players,
        pals,
        base_assignments,
    }))
}

async fn fetch_import_progress(
    state: &AppState,
    import_version_id: Uuid,
) -> Result<Option<ImportProgressEvent>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT
            iv.id,
            iv.status,
            iv.progress_phase,
            iv.progress_pct,
            iv.progress_message,
            iv.failed_error,
            iv.parse_metrics_json,
            iv.completed_at::text AS completed_at,
            (SELECT COUNT(*) FROM planner_players pp WHERE pp.import_version_id = iv.id) AS player_count,
            (SELECT COUNT(*) FROM planner_pals pp WHERE pp.import_version_id = iv.id) AS pal_count,
            (SELECT COUNT(*) FROM planner_base_assignments pba WHERE pba.import_version_id = iv.id) AS base_assignment_count
         FROM save_import_versions iv
         WHERE iv.id = $1",
    )
    .bind(import_version_id)
    .fetch_optional(&state.pool)
    .await?;

    Ok(row.map(|row| ImportProgressEvent {
        import_version_id: row.get("id"),
        status: row.get("status"),
        progress_phase: row.get("progress_phase"),
        progress_pct: row.get("progress_pct"),
        progress_message: row.get("progress_message"),
        failed_error: row.get("failed_error"),
        parse_metrics_json: row.get("parse_metrics_json"),
        completed_at: row.get("completed_at"),
        player_count: row.get("player_count"),
        pal_count: row.get("pal_count"),
        base_assignment_count: row.get("base_assignment_count"),
    }))
}
