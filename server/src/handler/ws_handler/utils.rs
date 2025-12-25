use uuid::Uuid;
use crate::config::AppState;
use crate::dtos::ServerResp;
use crate::errors::error::AppError;

pub fn send_event(state: &AppState, user_id: Uuid, event: ServerResp) {
    if let Some(sender) = state.channels.get(&user_id) {
        sender.send(event).ok();
    }
}

pub fn send_error(state: &AppState, user_id: Uuid, error: AppError) {
    send_event(
        state,
        user_id,
        ServerResp::Error {
            errors: error.to_api_errors(),
        },
    );
}
