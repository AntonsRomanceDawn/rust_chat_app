export interface User {
    id: string;
    username: string;
    role: 'admin' | 'user';
    created_at: string;
}

export interface LoginResp {
    access_token: string;
    refresh_token: string;
}

export interface RegisterResp {
    id: string;
    username: string;
    role: 'admin' | 'user';
    created_at: string;
}

export interface MessageInfo {
    message_id: string;
    author_username?: string;
    content: string;
    message_type: 'text' | 'file' | 'system';
    message_status: 'sent' | 'edited' | 'deleted';
    created_at: string;
}

export interface MemberInfo {
    username: string;
    joined_at: string;
}

export interface RoomInfo {
    room_id: string;
    room_name: string;
    last_message?: MessageInfo;
    unread_count: number;
}

export interface InvitationInfo {
    invitation_id: string;
    room_id: string;
    room_name: string;
    status: 'pending' | 'accepted' | 'declined';
    inviter_username: string;
    created_at: string;
}

export interface UserInfo {
    username: string;
    created_at: string;
}

// WebSocket Events
export type ClientReq =
    | { type: 'create_room'; name: string }
    | { type: 'join_room'; invitation_id: string }
    | { type: 'leave_room'; room_id: string }
    | { type: 'update_room'; room_id: string; name: string }
    | { type: 'delete_room'; room_id: string }
    | { type: 'get_room_info'; room_id: string }
    | { type: 'get_rooms_info' }
    | { type: 'invite'; room_id: string; username: string }
    | { type: 'decline_invitation'; invitation_id: string }
    | { type: 'get_pending_invitations' }
    | { type: 'send_message'; room_id: string; content: string }
    | { type: 'edit_message'; message_id: string; new_content: string }
    | { type: 'delete_message'; message_id: string }
    | { type: 'get_messages'; room_id: string; limit: number; offset: number }
    | { type: 'delete_account' }
    | { type: 'kick_member'; room_id: string; username: string }
    | { type: 'search_users'; query: string };

export type ServerResp =
    | { type: 'room_created'; room_id: string; room_name: string; created_at: string }
    | { type: 'room_joined'; invitation_id: string; room_id: string; room_name: string; admin_username: string; creator_username: string; created_at: string; joined_at: string }
    | { type: 'room_left'; room_id: string; room_name: string }
    | { type: 'room_updated'; room_id: string; room_name: string }
    | { type: 'room_deleted'; room_id: string; room_name: string }
    | { type: 'room_info'; room_id: string; room_name: string; admin_username: string; creator_username: string; members: MemberInfo[]; created_at: string }
    | { type: 'rooms_info'; rooms: RoomInfo[] }
    | { type: 'invitation_received'; invitation_id: string; room_id: string; room_name: string; inviter_username: string }
    | { type: 'invitation_sent'; invitation_id: string; room_id: string; room_name: string; invitee_username: string }
    | { type: 'invitation_declined'; invitation_id: string }
    | { type: 'invitee_declined'; invitation_id: string; room_id: string; room_name: string; invitee_username: string }
    | { type: 'pending_invitations'; pending_invitations: InvitationInfo[] }
    | { type: 'message_sent'; message_id: string; room_id: string; room_name: string; content: string; created_at: string; message_type: 'text' | 'file' | 'system'; message_status: 'sent' | 'edited' | 'deleted' }
    | { type: 'message_received'; message_id: string; room_id: string; room_name: string; author_username?: string; content: string; created_at: string; message_type: 'text' | 'file' | 'system'; message_status: 'sent' | 'edited' | 'deleted' }
    | { type: 'message_edited'; message_id: string; new_content: string }
    | { type: 'message_deleted'; message_id: string }
    | { type: 'message_history'; room_id: string; room_name: string; messages: MessageInfo[] }
    | { type: 'account_deleted'; user_id: string }
    | { type: 'member_kicked'; room_id: string; room_name: string; username: string }
    | { type: 'member_joined'; room_id: string; room_name: string; username: string; joined_at: string }
    | { type: 'users_found'; users: UserInfo[] }
    | { type: 'error'; errors: { code: string; message?: string }[] };
