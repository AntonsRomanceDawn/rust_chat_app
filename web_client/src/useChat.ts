import { useState, useEffect, useRef } from 'react';
import { ClientReq, ServerResp, RoomInfo, MessageInfo, InvitationInfo, UserInfo, MemberInfo } from './types';

const WS_URL = 'ws://localhost:3000/ws_handler'; // Or wss:// if using HTTPS

export interface RoomDetails {
    room_id: string;
    room_name: string;
    admin_username: string;
    creator_username: string;
    members: MemberInfo[];
    created_at: string;
}

export function useChat(token: string | null, refreshToken: () => Promise<string | null>) {
    const [socket, setSocket] = useState<WebSocket | null>(null);
    const [rooms, setRooms] = useState<RoomInfo[]>([]);
    const [currentRoom, setCurrentRoom] = useState<string | null>(null);
    const [messages, setMessages] = useState<Record<string, MessageInfo[]>>({});
    const [invitations, setInvitations] = useState<InvitationInfo[]>([]);
    const [searchResults, setSearchResults] = useState<UserInfo[]>([]);
    const [error, setError] = useState<string | null>(null);
    const [isConnected, setIsConnected] = useState(false);
    const [unreadCounts, setUnreadCounts] = useState<Record<string, number>>({});
    const [roomDetails, setRoomDetails] = useState<RoomDetails | null>(null);
    const [notification, setNotification] = useState<{ message: string, type: 'success' | 'error' } | null>(null);

    // Ref to access currentRoom inside the websocket callback without dependency issues
    const currentRoomRef = useRef<string | null>(null);
    useEffect(() => {
        currentRoomRef.current = currentRoom;
    }, [currentRoom]);

    useEffect(() => {
        if (!token) {
            setRooms([]);
            setCurrentRoom(null);
            setMessages({});
            setInvitations([]);
            setSearchResults([]);
            setIsConnected(false);
            setSocket(null);
            setUnreadCounts({});
            setRoomDetails(null);
            return;
        }

        let ws: WebSocket | null = null;
        let isUnmounted = false;
        let reconnectTimeout: number | null = null;

        const connect = () => {
            if (isUnmounted) return;
            ws = new WebSocket(`${WS_URL}?token=${token}`);

            ws.onopen = () => {
                if (isUnmounted) {
                    ws?.close();
                    return;
                }
                console.log('Connected to WebSocket');
                setIsConnected(true);
                setError(null);
                // Initial data fetch
                ws?.send(JSON.stringify({ type: 'get_rooms_info' }));
                ws?.send(JSON.stringify({ type: 'get_pending_invitations' }));
                if (currentRoomRef.current) {
                    ws?.send(JSON.stringify({ type: 'get_messages', room_id: currentRoomRef.current, limit: 50, offset: 0 }));
                }
            };

            ws.onmessage = (event) => {
                if (isUnmounted) return;
                try {
                    const data: ServerResp = JSON.parse(event.data);
                    handleServerMessage(data);
                } catch (e) {
                    console.error('Failed to parse message', e);
                }
            };

            ws.onclose = async (event) => {
                if (isUnmounted) return;
                console.log('Disconnected from WebSocket', event.code, event.reason);
                setIsConnected(false);
                setSocket(null);

                // If closed abnormally or due to auth error (though codes vary), try refresh
                // 1006 is abnormal closure (e.g. server died or connection dropped)
                // If server rejects handshake, it might close immediately.
                if (!isUnmounted) {
                    console.log('Attempting to reconnect...');
                    // Try to refresh token if it might be expired
                    // We don't know for sure if it's expired, but if we can't connect, it's a good guess
                    // However, we shouldn't loop infinitely refreshing.
                    // Simple strategy: Wait 3s, then try to refresh token, then let the token change trigger re-render
                    // BUT, if we refresh token, `token` prop changes, so this effect cleans up and runs again.
                    // So we just need to call refreshToken().

                    reconnectTimeout = setTimeout(async () => {
                        const newToken = await refreshToken();
                        if (!newToken) {
                            // Refresh failed (e.g. refresh token expired), user logged out by App
                            return;
                        }
                        // If refresh succeeded, `token` prop will update, triggering re-effect.
                        // If refresh returned same token (e.g. it wasn't expired but network error),
                        // we might need to force reconnect. But `refreshToken` implementation in App
                        // always gets a NEW access token from server. So `token` WILL change.
                    }, 3000);
                }
            };

            ws.onerror = (err) => {
                if (isUnmounted) return;
                console.error('WebSocket error', err);
                if (ws?.readyState !== WebSocket.OPEN) {
                    setError('Connection error');
                }
            };

            setSocket(ws);
        };

        connect();

        return () => {
            isUnmounted = true;
            if (reconnectTimeout) clearTimeout(reconnectTimeout);
            ws?.close();
        };
    }, [token]); // Re-run when token changes (e.g. after refresh)

    const handleServerMessage = (data: ServerResp) => {
        console.log('Received:', data);
        switch (data.type) {
            case 'rooms_info':
                setRooms(data.rooms);
                const counts: Record<string, number> = {};
                data.rooms.forEach(r => {
                    if (r.unread_count > 0) {
                        counts[r.room_id] = r.unread_count;
                    }
                });
                setUnreadCounts(counts);
                break;
            case 'room_created':
                setRooms(prev => [...prev, { room_id: data.room_id, room_name: data.room_name, unread_count: 0 }]);
                break;
            case 'room_joined':
                setRooms(prev => [...prev, { room_id: data.room_id, room_name: data.room_name, unread_count: 0 }]);
                // Remove invitation if it exists
                setInvitations(prev => prev.filter(inv => inv.invitation_id !== data.invitation_id));
                break;
            case 'room_left':
            case 'room_deleted':
                setRooms(prev => prev.filter(r => r.room_id !== data.room_id));
                if (currentRoomRef.current === data.room_id) setCurrentRoom(null);
                break;
            case 'pending_invitations':
                setInvitations(data.pending_invitations);
                break;
            case 'invitation_sent':
                setNotification({ message: `Invitation sent to ${data.invitee_username}`, type: 'success' });
                break;
            case 'invitation_received':
                setInvitations(prev => [...prev, {
                    invitation_id: data.invitation_id,
                    room_id: data.room_id,
                    room_name: data.room_name,
                    status: 'pending',
                    inviter_username: data.inviter_username,
                    created_at: new Date().toISOString()
                }]);
                break;
            case 'invitation_declined':
                setInvitations(prev => prev.filter(inv => inv.invitation_id !== data.invitation_id));
                break;
            case 'message_history':
                setMessages(prev => ({
                    ...prev,
                    [data.room_id]: data.messages
                }));
                break;
            case 'message_received':
            case 'message_sent':
                setMessages(prev => ({
                    ...prev,
                    [data.room_id]: [...(prev[data.room_id] || []), {
                        message_id: data.message_id,
                        author_username: data.type === 'message_sent' ? 'Me' : data.author_username, // Simplified
                        content: data.content,
                        created_at: data.created_at
                    }]
                }));
                if (data.type === 'message_received' && data.room_id !== currentRoomRef.current) {
                    setUnreadCounts(prev => ({
                        ...prev,
                        [data.room_id]: (prev[data.room_id] || 0) + 1
                    }));
                }
                break;
            case 'room_info':
                setRoomDetails({
                    room_id: data.room_id,
                    room_name: data.room_name,
                    admin_username: data.admin_username,
                    creator_username: data.creator_username,
                    members: data.members,
                    created_at: data.created_at
                });
                break;
            case 'users_found':
                setSearchResults(data.users);
                break;
            case 'error':
                setError(data.errors.map(e => e.code).join(', '));
                break;
        }
    };

    const send = (req: ClientReq) => {
        if (socket && socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify(req));
        } else {
            console.error('WebSocket not connected');
        }
    };

    const clearUnread = (roomId: string) => {
        setUnreadCounts(prev => {
            const newCounts = { ...prev };
            delete newCounts[roomId];
            return newCounts;
        });
    };

    return {
        rooms,
        currentRoom,
        setCurrentRoom,
        messages,
        invitations,
        searchResults,
        error,
        isConnected,
        send,
        unreadCounts,
        clearUnread,
        roomDetails,
        setRoomDetails,
        notification,
        setNotification
    };
}
