use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::mpsc;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    config::AppState,
    database::{
        invitations::InvitationRepository,
        keys::KeyRepository,
        models::{InvitationStatus, UserRole},
        refresh_token::RefreshTokenRepository,
        room_members::RoomMemberRepository,
        rooms::RoomRepository,
        user_messages::MessageRepository,
        users::UserRepository,
    },
    dtos::{
        ClientReq, InvitationInfo, KeyCountRespDto, LoginReqDto, LoginRespDto, MemberInfo,
        MessageInfo, OneTimePreKeyDto, PreKeyBundleRespDto, RefreshTokenReqDto,
        RefreshTokenRespDto, RegisterReqDto, RegisterRespDto, RoomInfo, ServerResp,
        SignedPreKeyDto, UploadKeysReqDto, UserInfo, WsParams,
    },
    errors::{
        error::{ApiErrorItem, HttpError},
        error_codes::{self, USER_NOT_FOUND},
    },
    utils::{
        hash::{hash_password, hash_refresh_token, verify_hashed_password},
        token::{
            extract_and_verify_token, generate_access_token, generate_refresh_token,
            verify_access_token,
        },
    },
};

pub fn handler(state: AppState) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh-token", post(refresh_token))
        .route("/keys", post(upload_keys))
        .route("/keys/status/count", get(get_key_count))
        .route("/keys/{username}", get(get_prekey_bundle))
        .route("/ws_handler", get(ws_handler))
        .with_state(state)
}

#[instrument(skip(state, body), fields(username = %body.username))]
async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterReqDto>,
) -> Result<Json<RegisterRespDto>, HttpError> {
    info!("Registering new user");
    body.validate().map_err(|e| HttpError::bad_request(e))?;

    let password_hash = hash_password(body.password)?;

    match state
        .db
        .insert_user(&body.username, &password_hash, UserRole::User)
        .await
    {
        Ok(Some(user)) => {
            info!("User registered successfully: {}", user.username);
            let register_response = RegisterRespDto {
                id: user.id,
                username: user.username,
                role: user.role,
                created_at: user.created_at,
            };
            Ok(Json::<RegisterRespDto>(register_response))
        }
        Ok(None) => {
            warn!(
                "Registration failed: Username {} already exists",
                body.username
            );
            Err(HttpError::bad_request([ApiErrorItem::new(
                error_codes::USERNAME_ALREADY_EXISTS,
                None,
            )]))
        }
        Err(e) => {
            error!("Registration failed: {:?}", e);
            return Err(HttpError::internal([ApiErrorItem::new(
                error_codes::INTERNAL_SERVER_ERROR,
                None,
            )]));
        }
    }
}

#[instrument(skip(state, body), fields(username = %body.username))]
async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginReqDto>,
) -> Result<Json<LoginRespDto>, HttpError> {
    info!("User logging in");
    body.validate().map_err(|e| HttpError::bad_request(e))?;
    let user = match state.db.get_user_by_username(&body.username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!(
                "Login failed: Wrong credentials for username {}",
                body.username
            );
            return Err(HttpError::bad_request([ApiErrorItem::new(
                error_codes::WRONG_CREDENTIALS,
                None,
            )]));
        }
        Err(e) => {
            error!("Login failed: {:?}", e);
            return Err(HttpError::internal([ApiErrorItem::new(
                error_codes::INTERNAL_SERVER_ERROR,
                None,
            )]));
        }
    };

    if !verify_hashed_password(&body.password, &user.password_hash)? {
        warn!(
            "Login failed: Wrong credentials for username {}",
            body.username
        );
        return Err(HttpError::bad_request([ApiErrorItem::new(
            error_codes::WRONG_CREDENTIALS,
            None,
        )]));
    }

    let access_token = generate_access_token(
        user.id,
        UserRole::User,
        state.config.jwt_secret.as_bytes(),
        state.config.access_expiry,
    )?;
    let refresh_token = generate_refresh_token()?;
    let refresh_token_hash = hash_refresh_token(&refresh_token);

    let _ = state
        .db
        .insert_refresh_token_by_hash(
            user.id,
            &refresh_token_hash,
            Duration::seconds(state.config.refresh_expiry),
        )
        .await
        .map_err(|e| {
            error!("Failed to insert refresh token: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?;

    let login_response = LoginRespDto {
        access_token,
        refresh_token,
    };
    Ok(Json::<LoginRespDto>(login_response))
}

#[instrument(skip(state, body))]
async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<RefreshTokenReqDto>,
) -> Result<Json<RefreshTokenRespDto>, HttpError> {
    info!("Refreshing token");
    let refresh_token_hash = hash_refresh_token(&body.refresh_token);
    let refresh_token = match state
        .db
        .get_refresh_token_by_hash(&refresh_token_hash)
        .await
    {
        Ok(Some(token)) => token,
        Ok(None) => {
            warn!(
                "Refresh token not found or expired for hash {}",
                refresh_token_hash
            );
            return Err(HttpError::unauthorized([ApiErrorItem::new(
                error_codes::SESSION_EXPIRED,
                None,
            )]));
        }
        Err(e) => {
            error!("Failed to get refresh token: {:?}", e);
            return Err(HttpError::internal([ApiErrorItem::new(
                error_codes::INTERNAL_SERVER_ERROR,
                None,
            )]));
        }
    };
    if refresh_token.expires_at < Utc::now() {
        warn!(
            "Refresh token not found or expired for hash {}",
            refresh_token_hash
        );
        return Err(HttpError::unauthorized([ApiErrorItem::new(
            error_codes::SESSION_EXPIRED,
            None,
        )]));
    }

    let user_id = refresh_token.user_id;

    let _ = state
        .db
        .delete_refresh_token_by_hash(&refresh_token_hash)
        .await
        .map_err(|e| {
            error!("Failed to delete refresh token: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?;

    let access_token = generate_access_token(
        user_id,
        UserRole::User,
        state.config.jwt_secret.as_bytes(),
        state.config.access_expiry,
    )?;
    let refresh_token = generate_refresh_token()?;
    let refresh_token_hash = hash_refresh_token(&refresh_token);

    let _ = state
        .db
        .insert_refresh_token_by_hash(
            user_id,
            &refresh_token_hash,
            Duration::seconds(state.config.refresh_expiry),
        )
        .await
        .map_err(|e| {
            error!("Failed to insert refresh token: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?;

    let refresh_response = RefreshTokenRespDto {
        access_token,
        refresh_token,
    };
    Ok(Json::<RefreshTokenRespDto>(refresh_response))
}

#[instrument(skip(state, body))]
async fn upload_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UploadKeysReqDto>,
) -> Result<(), HttpError> {
    info!("Uploading keys");
    let (user_id, _, _) = extract_and_verify_token(&headers, state.config.jwt_secret.as_bytes())?;

    let _ = state
        .db
        .upsert_identity_key(user_id, body.identity_key, body.registration_id)
        .await
        .map_err(|e| {
            error!("Failed to upsert identity key: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?;

    let _ = state
        .db
        .upsert_signed_prekey(
            user_id,
            body.signed_prekey.key_id,
            body.signed_prekey.public_key,
            body.signed_prekey.signature,
        )
        .await
        .map_err(|e| {
            error!("Failed to upsert signed prekey: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?;

    let ot_keys = body
        .one_time_prekeys
        .into_iter()
        .map(|k| (k.key_id, k.public_key))
        .collect();

    let _ = state
        .db
        .upload_one_time_prekeys(user_id, ot_keys)
        .await
        .map_err(|e| {
            error!("Failed to upload one-time prekeys: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?;

    Ok(())
}

#[instrument(skip(state))]
async fn get_key_count(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<KeyCountRespDto>, HttpError> {
    let (user_id, _, _) = extract_and_verify_token(&headers, state.config.jwt_secret.as_bytes())?;

    let count = state
        .db
        .get_prekey_bundle_counts(user_id)
        .await
        .map_err(|e| {
            error!("Failed to get key count: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?;

    Ok(Json(KeyCountRespDto { count }))
}

#[instrument(skip(state))]
async fn get_prekey_bundle(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> Result<Json<PreKeyBundleRespDto>, HttpError> {
    let _ = extract_and_verify_token(&headers, state.config.jwt_secret.as_bytes())?;

    let user = match state.db.get_user_by_username(&username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!("User not found: {}", username);
            return Err(HttpError::bad_request([ApiErrorItem::new(
                USER_NOT_FOUND,
                None,
            )]));
        }
        Err(e) => {
            error!("Failed to get user by username: {:?}", e);
            return Err(HttpError::internal([ApiErrorItem::new(
                error_codes::INTERNAL_SERVER_ERROR,
                None,
            )]));
        }
    };

    let user_id = user.id;

    let identity_key = state
        .db
        .get_identity_key(user_id)
        .await
        .map_err(|e| {
            error!("Failed to get identity key: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?
        .ok_or_else(|| {
            warn!("User {} has no identity key", user_id);
            HttpError::bad_request([ApiErrorItem::new(error_codes::USER_HAS_NO_KEYS, None)])
        })?;

    let signed_prekey = state
        .db
        .get_signed_prekey(user_id)
        .await
        .map_err(|e| {
            error!("Failed to get signed prekey: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?
        .ok_or_else(|| {
            warn!("User {} has no signed prekey", user_id);
            HttpError::bad_request([ApiErrorItem::new(error_codes::USER_HAS_NO_KEYS, None)])
        })?;

    let one_time_prekey = state
        .db
        .consume_one_time_prekey(user_id)
        .await
        .map_err(|e| {
            error!("Failed to consume one-time prekey: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?;

    Ok(Json(PreKeyBundleRespDto {
        identity_key: identity_key.identity_key,
        registration_id: identity_key.registration_id,
        signed_prekey: SignedPreKeyDto {
            key_id: signed_prekey.key_id,
            public_key: signed_prekey.public_key,
            signature: signed_prekey.signature,
        },
        one_time_prekey: one_time_prekey.map(|k| OneTimePreKeyDto {
            key_id: k.key_id,
            public_key: k.public_key,
        }),
    }))
}

#[instrument(skip(ws, state))]
async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, HttpError> {
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
                        let _ =
                            send_error(&state_clone, user_id, error_codes::INVALID_REQUEST_FORMAT);
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

#[instrument(skip(state), fields(user_id = %user_id))]
async fn create_room_response(state: &&AppState, user_id: Uuid, name: String) {
    info!("Creating room: {}", name);
    let username = match state.db.get_user_by_id(user_id).await {
        Ok(Some(user)) => user.username,
        _ => {
            error!("Failed to get user by id: {}", user_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state.db.create_room(&name, user_id, username).await {
        Ok(room) => {
            info!("Room created: {} with id {}", room.name, room.id);
            let _ = send_event(
                state,
                user_id,
                ServerResp::RoomCreated {
                    room_id: room.id,
                    room_name: room.name,
                    created_at: room.created_at,
                },
            );
        }
        Err(e) => {
            error!("Failed to create room: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn join_room_response(state: &&AppState, user_id: Uuid, invitation_id: Uuid) {
    info!(
        "User {} is attempting to join room with invitation {}",
        user_id, invitation_id
    );
    let room_id = match state.db.get_invitation_by_id(invitation_id).await {
        Ok(Some(invitation)) => invitation.room_id,
        Ok(None) => {
            warn!(
                "No pending invitation found for invitation id {}",
                invitation_id
            );
            let _ = send_error(state, user_id, error_codes::NO_PENDING_INVITATION);
            return;
        }
        Err(e) => {
            error!("Failed to get invitation by id: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let room = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room,
        _ => {
            error!("Failed to get room by id: {}", room_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let members = match state.db.get_members(room_id).await {
        Ok(members) => members,
        Err(e) => {
            error!("Failed to get room members: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    if members.iter().any(|m| m.user_id == user_id) {
        warn!("User {} is already a member of room {}", user_id, room_id);
        let _ = send_error(state, user_id, error_codes::ALREADY_ROOM_MEMBER);
        return;
    }

    let admin_username = match members
        .iter()
        .find(|m| m.user_id == room.admin_id)
        .map(|m| m.username.clone())
    {
        Some(username) => username,
        None => {
            error!("Admin user not found in members for room {}", room_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let creator_username = match state.db.get_user_by_id(room.creator_id).await {
        Ok(Some(user)) => user.username,
        _ => {
            error!("Failed to get creator user by id: {}", room.creator_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let invitee_username = match state.db.get_user_by_id(user_id).await {
        Ok(Some(user)) => user.username,
        _ => {
            error!("Failed to get invitee user by id: {}", user_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let now = Utc::now();

    let _ = state
        .db
        .consume_invitations_and_join_room(
            room_id,
            room.name.clone(),
            user_id,
            invitee_username.clone(),
            now,
        )
        .await
        .map_err(|e| {
            error!("Failed to consume invitation and join room: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
        });

    let event = ServerResp::MemberJoined {
        room_id,
        room_name: room.name.clone(),
        username: invitee_username.clone(),
        joined_at: now,
    };

    info!("User {} joined room {}", user_id, room_id);
    for member_id in members.into_iter().map(|m| m.user_id) {
        let _ = send_event(state, member_id, event.clone());
    }

    let _ = send_event(
        state,
        user_id,
        ServerResp::RoomJoined {
            invitation_id,
            room_id: room.id,
            room_name: room.name,
            admin_username,
            creator_username,
            created_at: room.created_at,
            joined_at: now,
        },
    );
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn leave_room_response(state: &&AppState, user_id: Uuid, room_id: Uuid) {
    info!("User {} is attempting to leave room {}", user_id, room_id);
    let _ = match state.db.get_room_by_id(room_id).await {
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, error_codes::ROOM_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        Ok(Some(_)) => {}
    };

    let _ = match state.db.is_member(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not a member of room {}", user_id, room_id);
            let _ = send_error(state, user_id, error_codes::NOT_ROOM_MEMBER);
            return;
        }
        Err(e) => {
            error!("Failed to check if user is a member of room: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        _ => {}
    };

    let _ = match state.db.leave_room(room_id, user_id).await {
        Ok(Some(room)) => {
            info!("User {} left room {}", user_id, room_id);
            let _ = send_event(
                state,
                user_id,
                ServerResp::RoomLeft {
                    room_id: room.id,
                    room_name: room.name,
                },
            );
        }
        _ => {
            error!("Failed to leave room: {}", room_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn update_room_response(state: &&AppState, user_id: Uuid, room_id: Uuid, name: String) {
    info!("User {} is attempting to update room {}", user_id, room_id);
    let _ = match state.db.get_room_by_id(room_id).await {
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, error_codes::ROOM_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        Ok(Some(_)) => {}
    };

    let _ = match state.db.is_admin(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not an admin of room {}", user_id, room_id);
            let _ = send_error(state, user_id, error_codes::NOT_ROOM_ADMIN);
            return;
        }
        Err(e) => {
            error!("Failed to check if user is an admin of room: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        _ => {}
    };

    let _ = match state.db.update_room_name(room_id, &name).await {
        Ok(Some(room)) => {
            info!("User {} updated room {}", user_id, room_id);
            let event = ServerResp::RoomUpdated {
                room_id: room.id,
                room_name: room.name.clone(),
            };
            if let Ok(members) = state.db.get_members(room_id).await {
                for member in members {
                    let _ = send_event(state, member.user_id, event.clone());
                }
            } else {
                error!("Failed to get members of room: {}", room_id);
                let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
                return;
            }
        }
        _ => {
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn delete_room_response(state: &&AppState, user_id: Uuid, room_id: Uuid) {
    info!("User {} is attempting to delete room {}", user_id, room_id);
    let _ = match state.db.get_room_by_id(room_id).await {
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, error_codes::ROOM_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        Ok(Some(_)) => {}
    };

    let _ = match state.db.is_admin(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not an admin of room {}", user_id, room_id);
            let _ = send_error(state, user_id, error_codes::NOT_ROOM_ADMIN);
            return;
        }
        Err(e) => {
            error!("Failed to check if user is an admin of room: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        _ => {}
    };

    let _ = match state.db.delete_room(room_id).await {
        Ok(Some(room)) => {
            info!("User {} deleted room {}", user_id, room_id);
            let event = ServerResp::RoomDeleted {
                room_id: room.id,
                room_name: room.name.clone(),
            };
            if let Ok(members) = state.db.get_members(room_id).await {
                for member in members {
                    let _ = send_event(state, member.user_id, event.clone());
                }
            } else {
                error!("Failed to get members of room: {}", room_id);
                let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
                return;
            }
        }
        _ => {
            error!("Failed to delete room: {}", room_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn get_room_info_response(state: &&AppState, user_id: Uuid, room_id: Uuid) {
    info!("User {} is requesting info for room {}", user_id, room_id);
    let room = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room,
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, error_codes::ROOM_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let members = match state.db.get_members(room_id).await {
        Ok(members) => members,
        Err(e) => {
            error!("Failed to get members of room: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let admin_username = match members
        .iter()
        .find(|m| m.user_id == room.admin_id)
        .map(|m| m.username.clone())
    {
        Some(username) => username,
        None => {
            error!("Admin user not found in members for room: {}", room_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let creator_username = match state.db.get_user_by_id(room.creator_id).await {
        Ok(Some(user)) => user.username,
        _ => {
            error!("Failed to get creator user by id: {}", room.creator_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let members_info = members
        .into_iter()
        .map(|m| MemberInfo {
            username: m.username,
            joined_at: m.joined_at,
        })
        .collect::<Vec<MemberInfo>>();

    info!("Sending room info for room {}", room_id);
    let _ = send_event(
        state,
        user_id,
        ServerResp::RoomInfo {
            room_id: room.id,
            room_name: room.name,
            admin_username,
            creator_username,
            members: members_info,
            created_at: room.created_at,
        },
    );
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn get_rooms_info_response(state: &&AppState, user_id: Uuid) {
    info!("User {} is requesting info for all their rooms", user_id);
    let _ = match state.db.get_rooms_info_for_user(user_id).await {
        Ok(rooms) => {
            info!("Sending rooms info to user {}", user_id);
            let rooms_info = rooms
                .into_iter()
                .map(|member| RoomInfo {
                    room_id: member.room_id,
                    room_name: member.room_name,
                    unread_count: member.unread_count,
                })
                .collect::<Vec<RoomInfo>>();
            let _ = send_event(state, user_id, ServerResp::RoomsInfo { rooms: rooms_info });
        }
        Err(e) => {
            error!("Failed to get user rooms for user {}: {:?}", user_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn invite_response(state: &&AppState, user_id: Uuid, room_id: Uuid, username: String) {
    info!(
        "User {} is attempting to invite {} to room {}",
        user_id, username, room_id
    );
    let room_name = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room.name,
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, error_codes::ROOM_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let invitee = match state.db.get_user_by_username(&username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!("Invite failed: User {} not found", username);
            let _ = send_error(state, user_id, error_codes::USER_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!("Failed to get user by username: {}: {:?}", username, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state.db.is_member(room_id, invitee.id).await {
        Ok(true) => {
            warn!(
                "Invite failed: User {} is already a member of room {}",
                username, room_id
            );
            let _ = send_error(state, user_id, error_codes::TARGET_ALREADY_ROOM_MEMBER);
            return;
        }
        Err(e) => {
            error!("Database error checking membership: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        _ => {}
    };

    let inviter = match state.db.get_user_by_id(user_id).await {
        Ok(Some(user)) => user,
        _ => {
            error!("Failed to get inviter user {}", user_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state.db.is_member(room_id, user_id).await {
        Ok(false) => {
            warn!(
                "Invite failed: Inviter {} is not a member of room {}",
                user_id, room_id
            );
            let _ = send_error(state, user_id, error_codes::NOT_ROOM_MEMBER);
            return;
        }
        Err(e) => {
            error!("Database error checking inviter membership: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        _ => {}
    };

    let _ = match state
        .db
        .create_invitation(
            room_id,
            room_name.clone(),
            invitee.id,
            invitee.username.clone(),
            inviter.id,
            inviter.username.clone(),
        )
        .await
    {
        Ok(Some(invitation)) => {
            info!(
                "Invitation created: {} invited {} to {}",
                inviter.username, invitee.username, room_name
            );
            let _ = send_event(
                state,
                user_id,
                ServerResp::InvitationSent {
                    invitation_id: invitation.id,
                    room_id: invitation.room_id,
                    room_name: invitation.room_name.clone(),
                    invitee_username: invitation.invitee_username,
                },
            );
            let _ = send_event(
                state,
                invitation.invitee_id,
                ServerResp::InvitationReceived {
                    invitation_id: invitation.id,
                    room_id: invitation.room_id,
                    room_name: invitation.room_name,
                    inviter_username: invitation.inviter_username,
                },
            );
        }
        Ok(None) => {
            warn!(
                "Invite failed: User {} has already been invited to room {}",
                username, room_id
            );
            let _ = send_error(state, user_id, error_codes::ALREADY_INVITED);
        }
        Err(e) => {
            error!("Database error creating invitation: {:?}", e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn decline_invitation_response(state: &&AppState, user_id: Uuid, invitation_id: Uuid) {
    info!(
        "User {} is attempting to decline invitation {}",
        user_id, invitation_id
    );
    let invitation = match state.db.get_invitation_by_id(invitation_id).await {
        Ok(Some(invitation)) => invitation,
        Ok(None) => {
            warn!("Invitation not found: {}", invitation_id);
            let _ = send_error(state, user_id, error_codes::INVITATION_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!(
                "Database error getting invitation by id {}: {:?}",
                invitation_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state.db.get_room_by_id(invitation.room_id).await {
        Ok(Some(_)) => {}
        _ => {
            error!("Failed to get room by id: {}", invitation.room_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state
        .db
        .update_invitation_status(invitation_id, InvitationStatus::Declined)
        .await
    {
        Ok(Some(invitation)) => {
            info!("User {} declined invitation {}", user_id, invitation_id);
            let _ = send_event(
                state,
                user_id,
                ServerResp::InvitationDeclined { invitation_id },
            );

            let _ = send_event(
                state,
                invitation.inviter_id,
                ServerResp::InviteeDeclined {
                    invitation_id,
                    room_id: invitation.room_id,
                    room_name: invitation.room_name,
                    invitee_username: invitation.invitee_username,
                },
            );
        }
        Ok(None) => {
            warn!(
                "No pending invitation found to decline for invitation id {}",
                invitation_id
            );
            let _ = send_error(state, user_id, error_codes::NO_PENDING_INVITATION);
            return;
        }
        Err(e) => {
            error!(
                "Database error updating invitation status for id {}: {:?}",
                invitation_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn get_pending_invitations_response(state: &&AppState, user_id: Uuid) {
    info!("User {} is requesting their pending invitations", user_id);
    let _ = match state.db.get_pending_invitations_for_user(user_id).await {
        Ok(invitations) => {
            let invitation_infos = invitations
                .into_iter()
                .map(|inv| InvitationInfo {
                    invitation_id: inv.id,
                    room_id: inv.room_id,
                    room_name: inv.room_name,
                    status: inv.status,
                    inviter_username: inv.inviter_username,
                    created_at: inv.created_at,
                })
                .collect::<Vec<InvitationInfo>>();
            info!(
                "Sending {} pending invitations to user {}",
                invitation_infos.len(),
                user_id
            );
            let _ = send_event(
                state,
                user_id,
                ServerResp::PendingInvitations {
                    pending_invitations: invitation_infos,
                },
            );
        }
        Err(e) => {
            error!(
                "Database error getting pending invitations for user {}: {:?}",
                user_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn send_message_response(
    state: &&AppState,
    user_id: Uuid,
    room_id: Uuid,
    content: String,
    message_type: Option<i32>,
) {
    info!("User {} is sending message to room {}", user_id, room_id);
    let message_type = message_type.unwrap_or(1); // Default to 1 (Ciphertext) if not provided
    let room_name = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room.name,
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, error_codes::ROOM_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!("Database error getting room by id {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state.db.is_member(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not a member of room {}", user_id, room_id);
            let _ = send_error(state, user_id, error_codes::NOT_ROOM_MEMBER);
            return;
        }
        Err(e) => {
            error!(
                "Database error checking membership for user {} in room {}: {:?}",
                user_id, room_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        _ => {}
    };

    let author = match state.db.get_user_by_id(user_id).await {
        Ok(Some(user)) => user,
        _ => {
            error!("Failed to get author user {}", user_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let message = match state
        .db
        .insert_message(
            room_id,
            room_name,
            author.id,
            author.username,
            &content,
            message_type,
        )
        .await
    {
        Ok(message) => message,
        Err(e) => {
            error!(
                "Database error inserting message in room {}: {:?}",
                room_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let member_ids = match state.db.get_members(room_id).await {
        Ok(members) => members
            .into_iter()
            .map(|m| m.user_id)
            .collect::<Vec<Uuid>>(),
        Err(e) => {
            error!(
                "Database error getting members for room {}: {:?}",
                room_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    info!(
        "Broadcasting message {} to room {} members",
        message.id, room_id
    );

    let event = ServerResp::MessageReceived {
        message_id: message.id,
        room_id: message.room_id,
        room_name: message.room_name.clone(),
        author_username: message.author_username,
        content: message.content.clone(),
        message_type: message.message_type,
        created_at: message.created_at,
    };

    for member_id in member_ids {
        if member_id == user_id {
            continue;
        }
        let _ = state
            .db
            .increment_unread_count(room_id, member_id)
            .await
            .map_err(|e| {
                error!(
                    "Database error incrementing unread count for user {} in room {}: {:?}",
                    member_id, room_id, e
                );
            });
        let _ = send_event(state, member_id, event.clone());
    }

    let _ = send_event(
        state,
        user_id,
        ServerResp::MessageSent {
            message_id: message.id,
            room_id: message.room_id,
            room_name: message.room_name,
            content: message.content,
            message_type: message.message_type,
            created_at: message.created_at,
        },
    );
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn edit_message_response(
    state: &&AppState,
    user_id: Uuid,
    message_id: Uuid,
    new_content: String,
) {
    info!("User {} is editing message {}", user_id, message_id);
    let message = match state.db.get_message_by_id(message_id).await {
        Ok(Some(message)) => message,
        Ok(None) => {
            warn!("Message not found: {}", message_id);
            let _ = send_error(state, user_id, error_codes::MESSAGE_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!(
                "Database error getting message by id {}: {:?}",
                message_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    if message.author_id != user_id {
        warn!(
            "User {} is not the author of message {}",
            user_id, message_id
        );
        let _ = send_error(state, user_id, error_codes::NOT_MESSAGE_AUTHOR);
        return;
    }

    let _ = match state
        .db
        .update_message_content(message_id, &new_content)
        .await
    {
        Ok(Some(updated_message)) => {
            info!("User {} edited message {}", user_id, message_id);
            let event = ServerResp::MessageEdited {
                message_id: updated_message.id,
                new_content: updated_message.content.clone(),
            };
            if let Ok(members) = state.db.get_members(updated_message.room_id).await {
                for member in members {
                    let _ = send_event(state, member.user_id, event.clone());
                }
            } else {
                error!(
                    "Database error getting members for room {}",
                    updated_message.room_id
                );
                let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
                return;
            }
        }
        _ => {
            error!(
                "Database error updating message content for message {}",
                message_id
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn delete_message_response(state: &&AppState, user_id: Uuid, message_id: Uuid) {
    info!("User {} is deleting message {}", user_id, message_id);
    let _ = match state.db.get_message_by_id(message_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            warn!("Message not found: {}", message_id);
            let _ = send_error(state, user_id, error_codes::MESSAGE_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!(
                "Database error getting message by id {}: {:?}",
                message_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state.db.delete_message(message_id).await {
        Ok(Some(message)) => {
            info!("User {} deleted message {}", user_id, message_id);
            let event = ServerResp::MessageDeleted { message_id };
            if let Ok(members) = state.db.get_members(message.room_id).await {
                for member in members {
                    let _ = send_event(state, member.user_id, event.clone());
                }
            } else {
                error!(
                    "Database error getting members for room {}",
                    message.room_id
                );
                let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
                return;
            }
        }
        _ => {
            error!("Database error deleting message {}", message_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn get_messages_response(
    state: &&AppState,
    user_id: Uuid,
    room_id: Uuid,
    limit: i64,
    offset: i64,
) {
    info!(
        "User {} is requesting messages for room {}",
        user_id, room_id
    );
    let room_name = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room.name,
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, error_codes::ROOM_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!("Database error getting room by id {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state
        .db
        .get_room_messages(room_id, user_id, limit, offset)
        .await
    {
        Ok(messages) => {
            if let Err(e) = state.db.reset_last_read_and_count(room_id, user_id).await {
                error!(
                    "Failed to update last_read_at for user {} in room {}: {:?}",
                    user_id, room_id, e
                );
            }

            let mut message_infos = Vec::new();
            for msg in messages {
                if let Ok(Some(author)) = state.db.get_user_by_id(msg.author_id).await {
                    message_infos.push(MessageInfo {
                        message_id: msg.id,
                        author_username: author.username,
                        content: msg.content,
                        message_type: msg.message_type,
                        created_at: msg.created_at,
                    });
                } else {
                    error!("Database error getting user by id {}", msg.author_id);
                    let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
                    return;
                }
            }
            info!(
                "Sending {} messages to user {} for room {}",
                message_infos.len(),
                user_id,
                room_id
            );
            let _ = send_event(
                state,
                user_id,
                ServerResp::MessageHistory {
                    room_id,
                    room_name,
                    messages: message_infos,
                },
            );
        }
        Err(_) => {
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn delete_account_response(state: &&AppState, user_id: Uuid) {
    info!("User {} is attempting to delete their account", user_id);
    let _ = match state.db.delete_user(user_id).await {
        Ok(Some(user)) => {
            info!("User {} deleted their account", user_id);
            let _ = send_event(
                state,
                user_id,
                ServerResp::AccountDeleted { user_id: user.id },
            );
        }
        _ => {
            error!("Failed to delete user account for user {}", user_id);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };
}
#[instrument(skip(state), fields(user_id = %user_id))]
async fn kick_member_response(state: &&AppState, user_id: Uuid, room_id: Uuid, username: String) {
    info!(
        "User {} is attempting to kick {} from room {}",
        user_id, username, room_id
    );
    let room_name = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room.name,
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, error_codes::ROOM_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!("Database error getting room by id {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state.db.is_admin(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not an admin of room {}", user_id, room_id);
            let _ = send_error(state, user_id, error_codes::NOT_ROOM_ADMIN);
            return;
        }
        Err(e) => {
            error!(
                "Database error checking admin status for user {} in room {}: {:?}",
                user_id, room_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
        _ => {}
    };

    let member = match state.db.get_user_by_username(&username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!("User not found: {}", username);
            let _ = send_error(state, user_id, error_codes::USER_NOT_FOUND);
            return;
        }
        Err(e) => {
            error!(
                "Database error getting user by username {}: {:?}",
                username, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let _ = match state.db.remove_member(room_id, member.id).await {
        Ok(Some(member)) => {
            info!("User {} was kicked from room {}", member.user_id, room_id);
            if let Ok(members) = state.db.get_members(room_id).await {
                let event = ServerResp::MemberKicked {
                    room_id: member.room_id,
                    room_name: member.room_name,
                    username: member.username,
                };
                for m in members {
                    let _ = send_event(state, m.user_id, event.clone());
                }
            } else {
                error!("Database error getting members for room {}", room_id);
                let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
                return;
            }
            let event = ServerResp::MemberKicked {
                room_id,
                room_name,
                username: username.clone(),
            };
            let _ = send_event(state, member.user_id, event);
        }
        Ok(None) => {
            warn!(
                "User {} is not a member of room {}",
                member.username, room_id
            );
            let _ = send_error(state, user_id, error_codes::TARGET_NOT_ROOM_MEMBER);
            return;
        }
        Err(e) => {
            error!(
                "Database error removing member {} from room {}: {:?}",
                member.username, room_id, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
async fn search_users_response(state: &&AppState, user_id: Uuid, query: String) {
    info!(
        "User {} is searching for users with query '{}'",
        user_id, query
    );
    let _ = match state.db.search_users(&query).await {
        Ok(users) => {
            info!(
                "Found {} users matching query '{}' for user {}",
                users.len(),
                query,
                user_id
            );
            let user_infos = users
                .into_iter()
                .map(|u| UserInfo {
                    username: u.username,
                    created_at: u.created_at,
                })
                .collect::<Vec<UserInfo>>();
            let _ = send_event(state, user_id, ServerResp::UsersFound { users: user_infos });
        }
        Err(e) => {
            error!(
                "Database error searching users with query '{}': {:?}",
                query, e
            );
            let _ = send_error(state, user_id, error_codes::INTERNAL_SERVER_ERROR);
        }
    };
}

fn send_event(state: &AppState, user_id: Uuid, event: ServerResp) {
    if let Some(sender) = state.channels.get(&user_id) {
        sender.send(event).ok();
    }
}

fn send_error(state: &AppState, user_id: Uuid, code: &'static str) {
    send_event(
        state,
        user_id,
        ServerResp::Error {
            errors: vec![ApiErrorItem::new(code, None)],
        },
    );
}
