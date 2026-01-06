import React, { useRef } from 'react';

interface UserInfo {
    username: string;
    created_at: string;
}

interface ChatHeaderProps {
    roomName: string;
    searchQuery: string;
    setSearchQuery: (query: string) => void;
    searchResults: UserInfo[];
    setSearchResults: (results: UserInfo[]) => void;
    handleSearchUsers: (e: React.FormEvent) => void;
    handleInvite: (username: string) => void;
    username: string | null;
    send: (msg: any) => void;
    currentRoom: string;
    setShowRoomInfo: (show: boolean) => void;
}

export const ChatHeader: React.FC<ChatHeaderProps> = ({
    roomName,
    searchQuery,
    setSearchQuery,
    searchResults,
    setSearchResults,
    handleSearchUsers,
    handleInvite,
    username,
    send,
    currentRoom,
    setShowRoomInfo,
}) => {
    const closeTimeoutRef = useRef<NodeJS.Timeout | null>(null);

    const handleMouseLeave = () => {
        closeTimeoutRef.current = setTimeout(() => {
            setSearchResults([]);
        }, 500);
    };

    const handleMouseEnter = () => {
        if (closeTimeoutRef.current) {
            clearTimeout(closeTimeoutRef.current);
            closeTimeoutRef.current = null;
        }
    };
    return (
        <div style={{
            background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
            padding: '16px 20px',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            color: 'white',
        }}>
            <div>
                <h2 style={{ margin: 0, fontSize: '20px', fontWeight: 600 }}>
                    #{roomName}
                </h2>
            </div>
            <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                <div
                    style={{ position: 'relative' }}
                    onMouseLeave={handleMouseLeave}
                    onMouseEnter={handleMouseEnter}
                >
                    <form onSubmit={handleSearchUsers}>
                        <div style={{ display: 'flex', gap: '6px' }}>
                            <input
                                type="text"
                                placeholder="Search to invite..."
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                style={{
                                    padding: '8px 12px',
                                    border: '1px solid rgba(255,255,255,0.3)',
                                    borderRadius: '6px',
                                    fontSize: '13px',
                                    width: '180px',
                                    background: 'rgba(255,255,255,0.2)',
                                    color: 'white',
                                    outline: 'none',
                                }}
                            />
                            <button
                                type="submit"
                                style={{
                                    background: 'rgba(255,255,255,0.2)',
                                    color: 'white',
                                    border: '1px solid rgba(255,255,255,0.3)',
                                    padding: '8px 16px',
                                    borderRadius: '6px',
                                    fontSize: '13px',
                                    fontWeight: 600,
                                    cursor: 'pointer',
                                }}
                            >
                                Search
                            </button>
                        </div>
                    </form>
                    {searchResults.length > 0 && (
                        <div style={{
                            position: 'absolute',
                            top: '100%',
                            left: 0,
                            background: 'white',
                            border: '1px solid #d1d5db',
                            borderRadius: '6px',
                            boxShadow: '0 4px 12px rgba(0,0,0,0.15)',
                            zIndex: 50,
                            minWidth: '250px',
                            marginTop: '6px',
                        }}>
                            {searchResults.filter((u) => u.username !== username).map((u) => (
                                <div
                                    key={u.username}
                                    style={{
                                        padding: '10px 14px',
                                        borderBottom: '1px solid #e5e7eb',
                                        display: 'flex',
                                        justifyContent: 'space-between',
                                        alignItems: 'center',
                                        fontSize: '13px',
                                        color: '#1f2937',
                                    }}
                                >
                                    <span>{u.username}</span>
                                    <button
                                        onClick={() => handleInvite(u.username)}
                                        style={{
                                            background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                                            color: 'white',
                                            border: 'none',
                                            padding: '6px 12px',
                                            borderRadius: '4px',
                                            fontSize: '12px',
                                            fontWeight: 600,
                                            cursor: 'pointer',
                                        }}
                                    >
                                        Invite
                                    </button>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
                <button
                    onClick={() => {
                        if (currentRoom) {
                            send({ type: 'get_room_info', room_id: currentRoom });
                            setShowRoomInfo(true);
                        }
                    }}
                    style={{
                        background: 'rgba(255,255,255,0.2)',
                        border: '1px solid rgba(255,255,255,0.3)',
                        fontSize: '13px',
                        cursor: 'pointer',
                        padding: '8px 16px',
                        borderRadius: '6px',
                        fontWeight: 600,
                        color: 'white',
                    }}
                >
                    Info
                </button>
            </div>
        </div>
    );
};
