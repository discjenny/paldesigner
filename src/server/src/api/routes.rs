use crate::AppState;
use crate::api::handlers;
use axum::{Router, routing::get};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        .with_state(state)
}
