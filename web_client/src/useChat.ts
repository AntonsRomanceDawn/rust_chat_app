import { useState, useEffect, useRef } from 'react';
import { ClientReq, ServerResp, RoomInfo, MessageInfo, InvitationInfo, UserInfo, MemberInfo } from './types';
import { SignalManager } from './lib/SignalManager';

// Compute WS URL dynamically: use env if provided, else derive from location with HTTPS/WSS
const WS_PATH = '/ws_handler';
const WS_URL = import.meta.env.VITE_WS_URL ?? `${window.location.protocol === 'https:' ? 'wss' : 'ws'}://${window.location.host}${WS_PATH}`;

export interface RoomDetails {
    room_id: string;
    room_name: string;
    admin_username: string;
    creator_username: string;
    members: MemberInfo[];
    created_at: string;
}

export function useChat(token: string | null, username: string | null, refreshToken: () => Promise<string | null>) {
    const [socket, setSocket] = useState<WebSocket | null>(null);
    const [rooms, setRooms] = useState<RoomInfo[]>([]);
    const [currentRoom, setCurrentRoom] = useState<string | null>(null);
    const [messages, setMessages] = useState<Record<string, MessageInfo[]>>({});
    const [invitations, setInvitations] = useState<InvitationInfo[]>([]);
    const [searchResults, setSearchResults] = useState<UserInfo[]>([]);
    const [isConnected, setIsConnected] = useState(false);
    const [roomDetails, setRoomDetails] = useState<RoomDetails | null>(null);
    const [notification, setNotification] = useState<{ message: string, type: 'success' | 'error' } | null>(null);
    const [signalManager, setSignalManager] = useState<SignalManager | null>(null);

    // Ref to access currentRoom inside the websocket callback without dependency issues
    const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
    const currentRoomRef = useRef<string | null>(null);
    useEffect(() => {
        currentRoomRef.current = currentRoom;
        if (currentRoom && socket && socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify({ type: 'get_room_info', room_id: currentRoom }));
            socket.send(JSON.stringify({ type: 'get_messages', room_id: currentRoom, limit: 50, offset: 0 }));
        }
    }, [currentRoom, socket]);

    useEffect(() => {
        if (token && username) {
            const manager = new SignalManager(username, token);
            manager.initialize().then(() => {
                console.log("Signal Manager Initialized");
                setSignalManager(manager);
            }).catch(e => {
                console.error("Failed to initialize Signal Manager", e);
            });
        } else {
            setSignalManager(null);
        }
    }, [token, username]);

    const signalManagerRef = useRef<SignalManager | null>(null);
    useEffect(() => {
        signalManagerRef.current = signalManager;
    }, [signalManager]);

    useEffect(() => {
        if (!token) {
            setRooms([]);
            setCurrentRoom(null);
            setMessages({});
            setInvitations([]);
            setSearchResults([]);
            setIsConnected(false);
            setSocket(null);
            setRoomDetails(null);
            return;
        }

        let ws: WebSocket | null = null;
        let isUnmounted = false;

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
                // Initial data fetch
                ws?.send(JSON.stringify({ type: 'get_rooms_info' }));
                ws?.send(JSON.stringify({ type: 'get_pending_invitations' }));
                if (currentRoomRef.current) {
                    ws?.send(JSON.stringify({ type: 'get_messages', room_id: currentRoomRef.current, limit: 50, offset: 0 }));
                }
            };

            ws.onmessage = async (event) => {
                if (isUnmounted) return;
                try {
                    const data: ServerResp = JSON.parse(event.data);
                    await handleServerMessage(data);
                } catch (e) {
                    console.error('Failed to parse message', e);
                }
            };

            ws.onclose = async (event) => {
                if (isUnmounted) return;
                console.log('Disconnected from WebSocket', event.code, event.reason);
                setIsConnected(false);
                setSocket(null);

                if (!isUnmounted) {
                    console.log('Attempting to reconnect...');
                    reconnectTimeoutRef.current = setTimeout(async () => {
                        const newToken = await refreshToken();
                        if (!newToken) {
                            return;
                        }
                    }, 3000);
                }
            };

            ws.onerror = (err) => {
                if (isUnmounted) return;
                console.error('WebSocket error', err);
                if (ws?.readyState !== WebSocket.OPEN) {
                    setNotification({ message: 'Connection error', type: 'error' });
                }
            };

            setSocket(ws);
        };

        connect();

        return () => {
            isUnmounted = true;
            if (reconnectTimeoutRef.current) clearTimeout(reconnectTimeoutRef.current);
            ws?.close();
        };
    }, [token]);

    const handleServerMessage = async (data: ServerResp) => {
        console.log('Received:', data);
        switch (data.type) {
            case 'rooms_info':
                setRooms(data.rooms);
                // Unread counts are now part of the room object, no need to maintain separate state map
                break;
            case 'room_created':
                setRooms(prev => [{ room_id: data.room_id, room_name: data.room_name, unread_count: 0 }, ...prev]);
                break;
            case 'room_joined':
                setRooms(prev => [{ room_id: data.room_id, room_name: data.room_name, unread_count: 0 }, ...prev]);
                setInvitations(prev => prev.filter(inv => inv.invitation_id !== data.invitation_id));
                setCurrentRoom(data.room_id);
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
                {
                    const decryptedMessages = await Promise.all(data.messages.map(async (msg) => {
                        let content = msg.content;
                        // if (signalManagerRef.current) {
                        //     content = await signalManagerRef.current.decryptMessage(msg.author_username, msg.content);
                        // }
                        return { ...msg, content, message_type: msg.message_type };
                    }));

                    setMessages(prev => {
                        const existing = prev[data.room_id] || [];
                        // Prepend new messages, avoiding duplicates based on message_id
                        const existingIds = new Set(existing.map(m => m.message_id));
                        const uniqueNew = decryptedMessages.filter(m => !existingIds.has(m.message_id));

                        // Since server returns [Old ... New] (ASC), and we request older info (Offset > 0),
                        // The fetched block is strictly OLDER than existing.
                        // So we prepend: [...fetched, ...existing]
                        // Note: If duplications happen due to race conditions or offset mismatch, filter handles it.

                        return {
                            ...prev,
                            [data.room_id]: [...uniqueNew, ...existing]
                        };
                    });
                }
                break;
            case 'message_received':
            case 'message_sent':
                {
                    console.log('MESSAGE_EVENT', data);
                    let content = data.content;
                    const author = data.type === 'message_sent' ? (username || 'Me') : data.author_username;
                    // if (signalManagerRef.current && data.type === 'message_received') {
                    //      content = await signalManagerRef.current.decryptMessage(author, data.content);
                    // }

                    const newMessage: MessageInfo = {
                        message_id: data.message_id,
                        author_username: author, // 'message_sent' doesn't have author_username in payload
                        content: content,
                        message_type: data.message_type,
                        status: data.status,
                        created_at: data.created_at
                    };

                    setMessages(prev => ({
                        ...prev,
                        [data.room_id]: [...(prev[data.room_id] || []), newMessage]
                    }));

                    // Update room list preview and unread count
                    setRooms(prev => {
                        const roomIndex = prev.findIndex(r => r.room_id === data.room_id);
                        if (roomIndex === -1) return prev;

                        const updatedRoom = { ...prev[roomIndex] };
                        updatedRoom.last_message = newMessage;

                        if (data.type === 'message_received' && data.room_id !== currentRoomRef.current) {
                            updatedRoom.unread_count += 1;
                        }

                        // Move updated room to top
                        const otherRooms = prev.filter(r => r.room_id !== data.room_id);
                        return [updatedRoom, ...otherRooms];
                    });
                }
                break;
            case 'message_edited':
                setMessages(prev => {
                    const newMessages = { ...prev };
                    for (const roomId in newMessages) {
                        const idx = newMessages[roomId].findIndex(m => m.message_id === data.message_id);
                        if (idx !== -1) {
                            newMessages[roomId] = newMessages[roomId].map(m =>
                                m.message_id === data.message_id ? { ...m, content: data.new_content, status: 'edited' } : m
                            );
                            // Update last message in room list if needed
                            setRooms(rooms => rooms.map(r => {
                                if (r.room_id === roomId && r.last_message?.message_id === data.message_id) {
                                    return { ...r, last_message: { ...r.last_message, content: data.new_content, status: 'edited' } };
                                }
                                return r;
                            }));
                            break;
                        }
                    }
                    return newMessages;
                });
                break;
            case 'message_deleted':
                setMessages(prev => {
                    const newMessages = { ...prev };
                    for (const roomId in newMessages) {
                        const idx = newMessages[roomId].findIndex(m => m.message_id === data.message_id);
                        if (idx !== -1) {
                            newMessages[roomId] = newMessages[roomId].map(m =>
                                m.message_id === data.message_id ? { ...m, content: '', status: 'deleted' } : m
                            );
                            // Update last message in room list if needed
                            setRooms(rooms => rooms.map(r => {
                                if (r.room_id === roomId && r.last_message?.message_id === data.message_id) {
                                    return { ...r, last_message: { ...r.last_message, content: '', status: 'deleted' } };
                                }
                                return r;
                            }));
                            break;
                        }
                    }
                    return newMessages;
                });
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
            case 'member_kicked':
                if (data.username === username) {
                    // I was kicked
                    setRooms(prev => prev.filter(r => r.room_id !== data.room_id));
                    if (currentRoomRef.current === data.room_id) {
                        setCurrentRoom(null);
                        setRoomDetails(null);
                    }
                    setNotification({ message: `You were kicked from ${data.room_name}`, type: 'error' });
                } else {
                    // Someone else was kicked
                    if (roomDetails && roomDetails.room_id === data.room_id) {
                        setRoomDetails(prev => prev ? {
                            ...prev,
                            members: prev.members.filter(m => m.username !== data.username)
                        } : null);
                    }
                    setNotification({ message: `${data.username} was kicked from ${data.room_name}`, type: 'success' });
                }
                break;
            case 'error':
                setNotification({ message: data.errors.map(e => e.code).join(', '), type: 'error' });
                break;
        }
    };

    const send = async (req: ClientReq) => {
        if (socket && socket.readyState === WebSocket.OPEN) {
            if (req.type === 'send_message' && signalManagerRef.current && roomDetails && roomDetails.room_id === req.room_id) {
                try {
                    const members = roomDetails.members.map(m => m.username);
                    const encrypted = await signalManagerRef.current.encryptGroupMessage(req.room_id, req.content, members);
                    const newReq = { ...req, content: encrypted.content };
                    socket.send(JSON.stringify(newReq));
                } catch (e) {
                    console.error("Encryption failed", e);
                    setNotification({ message: "Failed to encrypt message", type: 'error' });
                }
            } else {
                socket.send(JSON.stringify(req));
            }
        } else {
            console.error('WebSocket not connected');
        }
    };

    const clearUnread = (roomId: string) => {
        setRooms(prev => prev.map(r =>
            r.room_id === roomId ? { ...r, unread_count: 0 } : r
        ));
    };

    const loadMoreMessages = (roomId: string) => {
        const currentMsgs = messages[roomId] || [];
        // Only fetch if we have some messages (initial load logic handles 0)
        // or we rely on consumer to know.
        // Assuming limit=50.
        send({
            type: 'get_messages',
            room_id: roomId,
            limit: 50,
            offset: currentMsgs.length
        });
    };

    return {
        rooms,
        currentRoom,
        setCurrentRoom,
        messages,
        invitations,
        searchResults,
        setSearchResults,
        isConnected,
        send,
        clearUnread,
        roomDetails,
        setRoomDetails,
        notification,
        setNotification,
        loadMoreMessages
    };
}
