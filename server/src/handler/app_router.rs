use axum::{
    Router,
    routing::{get, post},
};

use crate::config::AppState;

use super::{
    auth_handler::{login, refresh_token, register},
    file_handler::{get_file, upload_file},
    keys_handler::{get_key_count, get_prekey_bundle, upload_keys},
    ws_handler::ws_router::ws_handler,
};

pub fn handler(state: AppState) -> Router {
    let api = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh-token", post(refresh_token))
        .route("/keys", post(upload_keys))
        .route("/keys/status/count", get(get_key_count))
        .route("/keys/{username}", get(get_prekey_bundle))
        .route("/files", post(upload_file))
        .route("/files/download", post(get_file));

    Router::new()
        .nest("/api", api)
        .route("/ws_handler", get(ws_handler))
        .with_state(state)
}
