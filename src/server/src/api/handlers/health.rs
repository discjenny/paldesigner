use crate::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use tracing::error;

#[derive(Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
}

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse { status: "ok" }))
}

pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(HealthResponse { status: "ready" })).into_response(),
        Err(err) => {
            error!(error = %err, "database readiness probe failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse {
                    status: "not_ready",
                }),
            )
                .into_response()
        }
    }
}
