use crate::AppState;
use crate::api::handlers;
use axum::{
    Router,
    routing::{get, post},
};

pub fn router(state: AppState) -> Router {
    let api_v1 = Router::new().route("/save/import-zip", post(handlers::import_zip::import_zip));

    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        .nest("/api/v1", api_v1)
        .with_state(state)
}
