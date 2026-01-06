import React from 'react';
import { RoomInfo } from '../types';

interface RoomListProps {
    rooms: RoomInfo[];
    currentRoom: string | null;
    unreadCounts: Record<string, number>;
    newRoomName: string;
    onNewRoomNameChange: (name: string) => void;
    onCreateRoom: (e: React.FormEvent) => void;
    onSelectRoom: (roomId: string) => void;
}

export const RoomList: React.FC<RoomListProps> = ({
    rooms,
    currentRoom,
    newRoomName,
    onNewRoomNameChange,
    onCreateRoom,
    onSelectRoom,
}) => {
    return (
        <>
            {/* Create Room */}
            <div style={{ padding: '12px', borderBottom: '1px solid #ccc', background: '#fff' }}>
                <form onSubmit={onCreateRoom} style={{ display: 'flex', gap: '8px' }}>
                    <input
                        type="text"
                        placeholder="New room..."
                        style={{
                            flex: 1,
                            padding: '8px',
                            border: '1px solid #ccc',
                            borderRadius: '4px',
                            fontSize: '12px',
                        }}
                        value={newRoomName}
                        onChange={(e) => onNewRoomNameChange(e.target.value)}
                    />
                    <button
                        type="submit"
                        style={{
                            background: '#2563eb',
                            color: 'white',
                            border: 'none',
                            padding: '8px 12px',
                            borderRadius: '4px',
                            fontWeight: 'bold',
                            cursor: 'pointer',
                        }}
                    >
                        +
                    </button>
                </form>
            </div>

            {/* Rooms List */}
            <div style={{ flex: 1, overflowY: 'auto' }}>
                {rooms.length === 0 ? (
                    <div style={{ textAlign: 'center', color: '#999', fontSize: '12px', marginTop: '32px' }}>
                        No rooms yet. Create one!
                    </div>
                ) : (
                    rooms.map((room) => (
                        <div
                            key={room.room_id}
                            onClick={() => onSelectRoom(room.room_id)}
                            className={`room-item ${currentRoom === room.room_id ? 'active' : ''}`}
                            style={{
                                padding: '12px',
                                borderBottom: '1px solid #f0f0f0',
                                backgroundColor: currentRoom === room.room_id ? '#dbeafe' : '#fff',
                                color: currentRoom === room.room_id ? '#2563eb' : '#333',
                                cursor: 'pointer',
                                display: 'flex',
                                flexDirection: 'column',
                                gap: '4px',
                                fontSize: '13px',
                                transition: 'background-color 0.2s',
                            }}
                        >
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                                <span style={{ fontWeight: 600 }}>#{room.room_name}</span>
                                {/* Use unread_count from the room object itself if provided, filtering overrides via props if needed */}
                                {room.unread_count > 0 && (
                                    <span style={{
                                        background: '#ef4444',
                                        color: 'white',
                                        fontSize: '10px',
                                        fontWeight: 'bold',
                                        padding: '2px 6px',
                                        borderRadius: '10px',
                                    }}>
                                        {room.unread_count}
                                    </span>
                                )}
                            </div>
                            <div style={{ fontSize: '11px', color: '#6b7280', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                                {room.last_message ? (
                                    <span>
                                        <span style={{ fontWeight: 600 }}>{room.last_message.author_username}: </span>
                                        {
                                            room.last_message.message_type === 'file' ? '[File]' :
                                                room.last_message.content
                                        }
                                    </span>
                                ) : (
                                    <span style={{ fontStyle: 'italic' }}>No messages yet</span>
                                )}
                            </div>
                        </div>
                    ))
                )}
            </div>
        </>
    );
};
