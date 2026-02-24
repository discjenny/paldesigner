use crate::AppState;
use crate::api::handlers;
use axum::{
    Router,
    routing::{get, post},
};

pub fn router(state: AppState) -> Router {
    let api_v1 = Router::new()
        .route("/save/import-zip", post(handlers::import_zip::import_zip))
        .route(
            "/save/import-versions",
            get(handlers::import_versions::list_import_versions),
        )
        .route(
            "/save/import-versions/{id}",
            get(handlers::import_versions::get_import_version),
        )
        .route(
            "/save/import-versions/{id}/events",
            get(handlers::import_versions::stream_import_version_events),
        )
        .route(
            "/save/import-versions/{id}/normalized",
            get(handlers::import_versions::get_normalized),
        );

    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        .nest("/api/v1", api_v1)
        .with_state(state)
}
