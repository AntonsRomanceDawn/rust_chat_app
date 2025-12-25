use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::mpsc;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{
    config::AppState,
    dtos::{ClientReq, ServerResp, WsParams},
    errors::error::AppError,
    utils::token::verify_access_token,
};

use super::{
    invitations::*,
    messages::*,
    rooms::*,
    users::*,
    utils::send_error,
};

#[instrument(skip(ws, state))]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    info!("WS connection attempt");
    let (user_id, _role, exp) =
        match verify_access_token(&params.token, state.config.jwt_secret.as_bytes()) {
            Ok(res) => res,
            Err(e) => {
                error!("WS token verification failed: {:?}", e);
                return Err(e);
            }
        };
    info!("WS connection accepted for user: {}", user_id);

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user_id, exp)))
}

#[instrument(skip(socket, state), fields(user_id = %user_id))]
async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid, exp: usize) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerResp>();

    state.channels.insert(user_id, tx);

    let mut send_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&event) {
                if sender.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<ClientReq>(&text) {
                    Ok(event) => {
                        handle_event(event, &state_clone, user_id).await;
                    }
                    Err(_) => {
                        let _ = send_error(&state_clone, user_id, AppError::InvalidRequestFormat);
                    }
                },
                Message::Binary(_) => {}
                Message::Close(_) => {
                    break;
                }
                Message::Ping(_) | Message::Pong(_) => {}
            }
        }
    });

    let now = Utc::now().timestamp() as usize;
    let duration_until_exp = if exp > now {
        std::time::Duration::from_secs((exp - now) as u64)
    } else {
        std::time::Duration::from_secs(0)
    };

    let mut expiration_task = tokio::spawn(async move {
        tokio::time::sleep(duration_until_exp).await;
    });

    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
            expiration_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
            expiration_task.abort();
        },
        _ = (&mut expiration_task) => {
            info!("Closing connection for user {} due to token expiration", user_id);
            send_task.abort();
            recv_task.abort();
        }
    };

    state.channels.remove(&user_id);
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn handle_event(event: ClientReq, state: &AppState, user_id: Uuid) {
    match event {
        ClientReq::CreateRoom { name } => create_room_response(&state, user_id, name).await,
        ClientReq::JoinRoom { invitation_id } => {
            join_room_response(&state, user_id, invitation_id).await
        }
        ClientReq::LeaveRoom { room_id } => leave_room_response(&state, user_id, room_id).await,
        ClientReq::UpdateRoom { room_id, name } => {
            update_room_response(&state, user_id, room_id, name).await
        }
        ClientReq::DeleteRoom { room_id } => delete_room_response(&state, user_id, room_id).await,
        ClientReq::GetRoomInfo { room_id } => {
            get_room_info_response(&state, user_id, room_id).await
        }
        ClientReq::GetRoomsInfo => get_rooms_info_response(&state, user_id).await,
        ClientReq::Invite { room_id, username } => {
            invite_response(&state, user_id, room_id, username).await
        }
        ClientReq::DeclineInvitation { invitation_id } => {
            decline_invitation_response(&state, user_id, invitation_id).await
        }
        ClientReq::GetPendingInvitations => get_pending_invitations_response(&state, user_id).await,
        ClientReq::SendMessage {
            room_id,
            content,
            message_type,
        } => send_message_response(&state, user_id, room_id, content, message_type).await,
        ClientReq::EditMessage {
            message_id,
            new_content,
        } => edit_message_response(&state, user_id, message_id, new_content).await,
        ClientReq::DeleteMessage { message_id } => {
            delete_message_response(&state, user_id, message_id).await
        }
        ClientReq::GetMessages {
            room_id,
            limit,
            offset,
        } => get_messages_response(&state, user_id, room_id, limit, offset).await,
        ClientReq::DeleteAccount => delete_account_response(&state, user_id).await,
        ClientReq::KickMember { room_id, username } => {
            kick_member_response(&state, user_id, room_id, username).await
        }
        ClientReq::SearchUsers { query } => search_users_response(&state, user_id, query).await,
    }
}
