pub mod config;
pub mod database;
pub mod dtos;
pub mod errors;
pub mod handler;
pub mod utils;

use axum::Router;
use config::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn create_app(state: AppState) -> Router {
    handler::app_router::handler(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
