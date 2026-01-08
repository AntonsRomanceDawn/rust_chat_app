import { useState, useEffect, useRef } from 'react';
import { SignalManager } from './signal/SignalManager';
import { ClientReq, ServerResp, RoomInfo, MessageInfo, InvitationInfo, UserInfo, MemberInfo } from './types';
import { messageDb } from './persistence/db';

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

export function useChat(token: string | null, username: string | null, refreshToken: () => Promise<string | null>, signalManager: SignalManager | null) {
    const [socket, setSocket] = useState<WebSocket | null>(null);
    const [rooms, setRooms] = useState<RoomInfo[]>([]);
    const [currentRoom, setCurrentRoom] = useState<string | null>(null);
    const [messages, setMessages] = useState<Record<string, MessageInfo[]>>({});
    const [invitations, setInvitations] = useState<InvitationInfo[]>([]);
    const [searchResults, setSearchResults] = useState<UserInfo[]>([]);
    const [isConnected, setIsConnected] = useState(false);
    const [roomDetails, setRoomDetails] = useState<RoomDetails | null>(null);
    const [notification, setNotification] = useState<{ message: string, type: 'success' | 'error' | 'info' } | null>(null);

    const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
    const currentRoomRef = useRef<string | null>(null);
    const signalManagerRef = useRef(signalManager);

    useEffect(() => {
        signalManagerRef.current = signalManager;
    }, [signalManager]);

    useEffect(() => {
        currentRoomRef.current = currentRoom;
        if (currentRoom) {
            setRoomDetails(null);
        }
        if (currentRoom && socket && socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify({ type: 'get_room_info', room_id: currentRoom }));
            socket.send(JSON.stringify({ type: 'get_messages', room_id: currentRoom, limit: 50, offset: 0 }));
        }
    }, [currentRoom, socket]);

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
                {
                    const sm = signalManagerRef.current;
                    const processedRooms = await Promise.all(
                        (data.rooms as RoomInfo[]).map(async (r) => {
                            if (r.last_message) {
                                const stored = await messageDb.getMessage(r.last_message.message_id);
                                if (stored) {
                                    return {
                                        ...r,
                                        last_message: { ...r.last_message, content: stored.content }
                                    };
                                }

                                let content = r.last_message.content;
                                if (r.last_message.message_type === 'text' && sm) {
                                    try {
                                        content = await sm.decryptGroupMessage(r.last_message.author_username || 'Unknown', content);
                                        await messageDb.saveMessage({
                                            ...r.last_message,
                                            content,
                                            room_id: r.room_id
                                        });
                                    } catch (e) {
                                        content = "Message";
                                    }
                                }
                                return { ...r, last_message: { ...r.last_message, content } };
                            }
                            return r;
                        })
                    );
                    setRooms(processedRooms);
                }
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
            case 'invitation_room_deleted':
                setInvitations(prev => prev.filter(inv => inv.invitation_id !== data.invitation_id));
                setNotification({ message: `Invitation for room "${data.room_name}" was removed because the room was deleted.`, type: 'info' });
                break;
            case 'invitee_declined':
                setNotification({ message: `${data.invitee_username} declined your invitation to ${data.room_name}`, type: 'info' });
                break;
            case 'message_history':
                {
                    const sm = signalManagerRef.current;
                    const decryptedMessages = await Promise.all(data.messages.map(async (msg) => {
                        const stored = await messageDb.getMessage(msg.message_id);
                        if (stored) {
                            return { ...msg, content: stored.content, message_type: msg.message_type };
                        }

                        let content = msg.content;
                        if (msg.message_type === 'text' && sm) {
                            try {
                                content = await sm.decryptGroupMessage(msg.author_username || 'Unknown', msg.content);
                                await messageDb.saveMessage({
                                    ...msg,
                                    content,
                                    room_id: data.room_id
                                });
                            } catch (e) {
                                content = "[Decryption Failed - Session Expired]";
                            }
                        }
                        return { ...msg, content, message_type: msg.message_type };
                    }));

                    if (decryptedMessages.length > 0) {
                        const lastMsg = decryptedMessages[decryptedMessages.length - 1];

                        setRooms(prev => prev.map(r => {
                            if (r.room_id === data.room_id) {
                                if (!r.last_message || new Date(r.last_message.created_at) < new Date(lastMsg.created_at)) {
                                    return { ...r, last_message: lastMsg };
                                }
                            }
                            return r;
                        }));
                    }


                    setMessages(prev => {
                        const existing = prev[data.room_id] || [];
                        const existingIds = new Set(existing.map(m => m.message_id));
                        const uniqueNew = decryptedMessages.filter(m => !existingIds.has(m.message_id));

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
                    let content = data.content;
                    let author = (data as any).author_username || username || 'Unknown';
                    const sm = signalManagerRef.current;

                    if (data.message_type === 'text' && sm) {
                        try {
                            content = await sm.decryptGroupMessage(author, data.content);
                        } catch (e) {
                            content = "[Decryption Failed - Session Expired]";
                        }
                    }

                    const newMessage: MessageInfo = {
                        message_id: data.message_id,
                        author_username: author,
                        content: content,
                        message_type: data.message_type,
                        message_status: data.message_status,
                        created_at: data.created_at
                    };

                    if (data.message_type === 'text' && content !== "[Decryption Error]") {
                        await messageDb.saveMessage({
                            ...newMessage,
                            room_id: data.room_id
                        });
                    }

                    setMessages(prev => ({
                        ...prev,
                        [data.room_id]: [...(prev[data.room_id] || []), newMessage]
                    }));

                    setRooms(prev => {
                        const roomIndex = prev.findIndex(r => r.room_id === data.room_id);
                        if (roomIndex === -1) return prev;

                        const updatedRoom = { ...prev[roomIndex] };
                        updatedRoom.last_message = newMessage;

                        if (data.type === 'message_received' && data.room_id !== currentRoomRef.current) {
                            updatedRoom.unread_count += 1;
                        }

                        const otherRooms = prev.filter(r => r.room_id !== data.room_id);
                        return [updatedRoom, ...otherRooms];
                    });
                }
                break;
            case 'message_edited':
                {
                    let decryptedContent = data.new_content;
                    let author: string | undefined;
                    for (const rid in messages) {
                        const existing = messages[rid].find(m => m.message_id === data.message_id);
                        if (existing) {
                            author = existing.author_username;
                            break;
                        }
                    }
                    author = author || 'Unknown';

                    setMessages(prev => {
                        const newMessages = { ...prev };
                        for (const roomId in newMessages) {
                            const idx = newMessages[roomId].findIndex(m => m.message_id === data.message_id);
                            if (idx !== -1) {
                                newMessages[roomId] = newMessages[roomId].map(m =>
                                    m.message_id === data.message_id ? { ...m, content: decryptedContent, message_status: 'edited' } : m
                                );

                                setRooms(rooms => rooms.map(r => {
                                    if (r.room_id === roomId && r.last_message?.message_id === data.message_id) {
                                        return { ...r, last_message: { ...r.last_message, content: decryptedContent, message_status: 'edited' } };
                                    }
                                    return r;
                                }));
                                break;
                            }
                        }
                        return newMessages;
                    });
                }
                break;
            case 'message_deleted':
                setMessages(prev => {
                    const newMessages = { ...prev };
                    for (const roomId in newMessages) {
                        const idx = newMessages[roomId].findIndex(m => m.message_id === data.message_id);
                        if (idx !== -1) {
                            newMessages[roomId] = newMessages[roomId].map(m =>
                                m.message_id === data.message_id ? { ...m, content: '', message_status: 'deleted' } : m
                            );
                            // Update last message in room list if needed
                            setRooms(rooms => rooms.map(r => {
                                if (r.room_id === roomId && r.last_message?.message_id === data.message_id) {
                                    return { ...r, last_message: { ...r.last_message, content: '', message_status: 'deleted' } };
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
            case 'member_joined':
                if (currentRoomRef.current === data.room_id) {
                    setRoomDetails(prev => {
                        if (!prev || prev.room_id !== data.room_id) return prev;
                        if (prev.members.some(m => m.username === data.username)) return prev;
                        return {
                            ...prev,
                            members: [...prev.members, { username: data.username, joined_at: data.joined_at }]
                        };
                    });
                }
                setNotification({ message: `${data.username} joined ${data.room_name}`, type: 'success' });
                break;
            case 'member_kicked':
                if (data.username === username) {
                    // I was kicked
                    if (currentRoomRef.current === data.room_id) {
                    }
                    setNotification({ message: `You were kicked from ${data.room_name}`, type: 'error' });
                } else {
                    // Someone else was kicked
                    setRoomDetails(prev => {
                        if (prev && prev.room_id === data.room_id) {
                            return {
                                ...prev,
                                members: prev.members.filter(m => m.username !== data.username)
                            };
                        }
                        return prev;
                    });
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
            if (req.type === 'send_message') {
                try {
                    let content = req.content;
                    if (signalManager && (req.message_type === undefined || req.message_type === 'text')) { // Default is text

                        if (!roomDetails || roomDetails.room_id !== req.room_id) {
                            setNotification({ message: "Cannot encrypt: Room participant list not loaded. Please re-select the room.", type: 'error' });
                            return;
                        }

                        const recipients = roomDetails.members.map(m => m.username);
                        content = await signalManager.encryptGroupMessage(recipients, req.content);
                    }

                    const newReq = { ...req, content: content };
                    socket.send(JSON.stringify(newReq));
                } catch (e) {
                    console.error("Encryption/Send failed", e);
                    setNotification({ message: "Failed to send message", type: 'error' });
                }
            } else if (req.type === 'edit_message') {
                try {
                    const newReq = { ...req, new_content: req.new_content };
                    socket.send(JSON.stringify(newReq));
                } catch (e) {
                    // console.error("Encryption failed", e);
                    // setNotification({ message: "Failed to encrypt message edit", type: 'error' });
                }
            } else {
                socket.send(JSON.stringify(req));
            }
        }
    };

    const clearUnread = (roomId: string) => {
        setRooms(prev => prev.map(r =>
            r.room_id === roomId ? { ...r, unread_count: 0 } : r
        ));
    };

    const loadMoreMessages = (roomId: string) => {
        const currentMsgs = messages[roomId] || [];
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
