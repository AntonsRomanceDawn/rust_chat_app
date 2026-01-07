import React from 'react';

interface Room {
    room_id: string;
    room_name: string;
    unread_count: number;
    last_message?: {
        author_username?: string;
        content: string;
        message_type: string;
    };
}

interface Invitation {
    invitation_id: string;
    inviter_username: string;
    room_name: string;
}

interface SidebarProps {
    activeTab: 'rooms' | 'invitations';
    setActiveTab: (tab: 'rooms' | 'invitations') => void;
    rooms: Room[];
    invitations: Invitation[];
    currentRoom: string | null;
    setCurrentRoom: (roomId: string) => void;
    newRoomName: string;
    setNewRoomName: (name: string) => void;
    handleCreateRoom: (e: React.FormEvent) => void;
    clearUnread: (roomId: string) => void;
    send: (msg: any) => void;
    isConnected: boolean;
    handleLogout: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
    activeTab,
    setActiveTab,
    rooms,
    invitations,
    currentRoom,
    setCurrentRoom,
    newRoomName,
    setNewRoomName,
    handleCreateRoom,
    clearUnread,
    send,
    isConnected,
    handleLogout,
}) => {
    return (
        <div style={{
            width: '300px',
            background: 'rgba(255, 255, 255, 0.95)',
            borderRadius: '12px',
            boxShadow: '0 8px 32px rgba(0, 0, 0, 0.1)',
            display: 'flex',
            flexDirection: 'column',
            overflow: 'hidden',
            height: '100%', // Match parent height
        }}>
            {/* Header with Connection and Logout */}
            <div style={{
                background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                padding: '0 20px',
                height: '70px', // Fixed height to match ChatHeader
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                color: 'white',
                flexShrink: 0,
            }}>
                <span style={{
                    fontSize: '12px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    opacity: 0.95,
                }}>
                    <span style={{
                        width: '8px',
                        height: '8px',
                        borderRadius: '50%',
                        background: isConnected ? '#10b981' : '#ef4444',
                    }} />
                    {isConnected ? 'Connected' : 'Disconnected'}
                </span>
                <button
                    onClick={handleLogout}
                    style={{
                        background: '#ef4444',
                        border: 'none',
                        color: 'white',
                        padding: '6px 12px',
                        borderRadius: '6px',
                        fontSize: '12px',
                        fontWeight: 600,
                        cursor: 'pointer',
                        transition: 'background 0.2s',
                    }}
                    onMouseEnter={(e) => e.currentTarget.style.background = '#dc2626'}
                    onMouseLeave={(e) => e.currentTarget.style.background = '#ef4444'}
                >
                    Logout
                </button>
            </div>

            {/* Tabs */}
            <div style={{ display: 'flex', gap: '8px', padding: '12px' }}>
                <button
                    onClick={() => setActiveTab('rooms')}
                    style={{
                        flex: 1,
                        padding: '10px',
                        fontSize: '13px',
                        fontWeight: 600,
                        background: activeTab === 'rooms' ? 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)' : '#e5e7eb',
                        color: activeTab === 'rooms' ? 'white' : '#374151',
                        border: 'none',
                        borderRadius: '6px',
                        cursor: 'pointer',
                        transition: 'all 0.2s',
                    }}
                >
                    Rooms
                </button>
                <button
                    onClick={() => setActiveTab('invitations')}
                    style={{
                        flex: 1,
                        padding: '10px',
                        fontSize: '13px',
                        fontWeight: 600,
                        background: activeTab === 'invitations' ? 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)' : '#e5e7eb',
                        color: activeTab === 'invitations' ? 'white' : '#374151',
                        border: 'none',
                        borderRadius: '6px',
                        cursor: 'pointer',
                        position: 'relative',
                        transition: 'all 0.2s',
                    }}
                >
                    Invitations
                    {invitations.length > 0 && (
                        <span style={{
                            position: 'absolute',
                            top: '4px',
                            right: '8px',
                            background: '#ef4444',
                            color: 'white',
                            fontSize: '10px',
                            fontWeight: 'bold',
                            padding: '2px 6px',
                            borderRadius: '10px',
                        }}>
                            {invitations.length}
                        </span>
                    )}
                </button>
            </div>

            <div style={{ flex: 1, overflowY: 'auto' }}>
                {activeTab === 'rooms' ? (
                    <>
                        {/* Create Room Form */}
                        <div style={{ padding: '0 12px 12px' }}>
                            <form onSubmit={handleCreateRoom} style={{ display: 'flex', gap: '6px' }}>
                                <input
                                    type="text"
                                    placeholder="New room..."
                                    style={{
                                        flex: 1,
                                        padding: '8px 12px',
                                        border: '1px solid #d1d5db',
                                        borderRadius: '6px',
                                        fontSize: '13px',
                                        outline: 'none',
                                    }}
                                    value={newRoomName}
                                    onChange={(e) => setNewRoomName(e.target.value)}
                                />
                                <button
                                    type="submit"
                                    style={{
                                        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                                        color: 'white',
                                        border: 'none',
                                        padding: '8px 16px',
                                        borderRadius: '6px',
                                        fontWeight: 'bold',
                                        cursor: 'pointer',
                                        fontSize: '16px',
                                    }}
                                >
                                    +
                                </button>
                            </form>
                        </div>

                        {/* Rooms List */}
                        {rooms.length === 0 ? (
                            <div style={{ textAlign: 'center', color: '#999', fontSize: '13px', padding: '20px' }}>
                                No rooms yet. Create one!
                            </div>
                        ) : (
                            rooms.map((room) => (
                                <div
                                    key={room.room_id}
                                    onClick={() => {
                                        setCurrentRoom(room.room_id);
                                        clearUnread(room.room_id);
                                        send({
                                            type: 'get_messages',
                                            room_id: room.room_id,
                                            limit: 50,
                                            offset: 0,
                                        });
                                    }}
                                    style={{
                                        padding: '12px 16px',
                                        borderBottom: '1px solid #e5e7eb',
                                        cursor: 'pointer',
                                        background: currentRoom === room.room_id ? '#ede9fe' : 'transparent',
                                        transition: 'background 0.2s',
                                    }}
                                    onMouseEnter={(e) => e.currentTarget.style.background = currentRoom === room.room_id ? '#ede9fe' : '#f9fafb'}
                                    onMouseLeave={(e) => e.currentTarget.style.background = currentRoom === room.room_id ? '#ede9fe' : 'transparent'}
                                >
                                    <div style={{ fontWeight: 600, fontSize: '14px', color: '#1f2937', marginBottom: '4px' }}>
                                        {room.room_name}
                                    </div>
                                    <div style={{
                                        display: 'flex',
                                        justifyContent: 'space-between',
                                        alignItems: 'center',
                                    }}>
                                        <div style={{ fontSize: '12px', color: '#6b7280', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: '200px' }}>
                                            {room.last_message ? (
                                                <span>
                                                    {room.last_message.message_type === 'system' ? null : <span style={{ fontWeight: 900 }}>{room.last_message.author_username}: </span>}
                                                    {(() => {
                                                        try {
                                                            // Check for system message type first if available
                                                            if (room.last_message.message_type === 'system') {
                                                                const parsed = JSON.parse(room.last_message.content);
                                                                switch (parsed.type) {
                                                                    case 'joined': return `${parsed.username} joined`;
                                                                    case 'left': return `${parsed.username} left`;
                                                                    case 'kicked': return `${parsed.username} kicked`;
                                                                    default: return 'System message';
                                                                }
                                                            }

                                                            const parsed = JSON.parse(room.last_message.content);
                                                            if (parsed.type === 'file' && parsed.filename) {
                                                                return `📁 ${parsed.filename}`;
                                                            }
                                                            // Fallback for sniffed system messages if type isn't set on last_message object
                                                            if (parsed.type === 'joined' && parsed.username) return `${parsed.username} joined`;
                                                            if (parsed.type === 'left' && parsed.username) return `${parsed.username} left`;
                                                            if (parsed.type === 'kicked' && parsed.username) return `${parsed.username} kicked`;

                                                        } catch { }
                                                        return room.last_message.content;
                                                    })()}
                                                </span>
                                            ) : (
                                                <span style={{ fontStyle: 'italic' }}>No messages yet</span>
                                            )}
                                        </div>
                                        {room.unread_count > 0 && (
                                            <span style={{
                                                background: '#ef4444',
                                                color: 'white',
                                                fontSize: '10px',
                                                fontWeight: 'bold',
                                                padding: '2px 6px',
                                                borderRadius: '10px',
                                                minWidth: '18px',
                                                textAlign: 'center',
                                            }}>
                                                {room.unread_count}
                                            </span>
                                        )}
                                    </div>
                                </div>
                            ))
                        )}
                    </>
                ) : (
                    <>
                        {/* Invitations List */}
                        {invitations.length === 0 ? (
                            <div style={{ textAlign: 'center', color: '#999', fontSize: '13px', padding: '20px' }}>
                                No pending invitations
                            </div>
                        ) : (
                            invitations.map((inv) => (
                                <div
                                    key={inv.invitation_id}
                                    style={{
                                        padding: '12px 16px',
                                        borderBottom: '1px solid #e5e7eb',
                                    }}
                                >
                                    <p style={{ fontSize: '13px', margin: '0 0 10px 0', lineHeight: '1.5' }}>
                                        <strong style={{ color: '#667eea' }}>{inv.inviter_username}</strong> invited you to <strong>{inv.room_name}</strong>
                                    </p>
                                    <div style={{ display: 'flex', gap: '6px' }}>
                                        <button
                                            onClick={() =>
                                                send({
                                                    type: 'join_room',
                                                    invitation_id: inv.invitation_id,
                                                })
                                            }
                                            style={{
                                                flex: 1,
                                                background: '#16a34a',
                                                color: 'white',
                                                border: 'none',
                                                padding: '8px',
                                                borderRadius: '6px',
                                                fontSize: '12px',
                                                fontWeight: 600,
                                                cursor: 'pointer',
                                            }}
                                        >
                                            Accept
                                        </button>
                                        <button
                                            onClick={() =>
                                                send({
                                                    type: 'decline_invitation',
                                                    invitation_id: inv.invitation_id,
                                                })
                                            }
                                            style={{
                                                flex: 1,
                                                background: '#dc2626',
                                                color: 'white',
                                                border: 'none',
                                                padding: '8px',
                                                borderRadius: '6px',
                                                fontSize: '12px',
                                                fontWeight: 600,
                                                cursor: 'pointer',
                                            }}
                                        >
                                            Decline
                                        </button>
                                    </div>
                                </div>
                            ))
                        )}
                    </>
                )}
            </div>
        </div>
    );
};
