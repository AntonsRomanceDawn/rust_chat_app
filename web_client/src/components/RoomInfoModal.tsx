import React from 'react';

interface RoomMember {
    username: string;
    joined_at: string;
}

interface RoomDetails {
    room_id: string;
    room_name: string;
    creator_username: string;
    admin_username: string;
    created_at: string;
    members: RoomMember[];
}

interface RoomInfoModalProps {
    roomDetails: RoomDetails | null;
    setShowRoomInfo: (show: boolean) => void;
    setRoomDetails: (details: RoomDetails | null) => void;
    username: string | null;
    send: (msg: any) => void;
}

export const RoomInfoModal: React.FC<RoomInfoModalProps> = ({
    roomDetails,
    setShowRoomInfo,
    setRoomDetails,
    username,
    send,
}) => {
    if (!roomDetails) return null;

    return (
        <div style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(0,0,0,0.5)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 50,
        }}>
            <div style={{
                background: '#fff',
                padding: '24px',
                borderRadius: '8px',
                boxShadow: '0 10px 25px rgba(0,0,0,0.2)',
                width: '384px',
                maxHeight: '80vh',
                overflowY: 'auto',
            }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
                    <h2 style={{ fontSize: '18px', fontWeight: 'bold', margin: 0 }}>Room Info</h2>
                    <button
                        onClick={() => {
                            setShowRoomInfo(false);
                            setRoomDetails(null);
                        }}
                        style={{
                            background: 'none',
                            border: 'none',
                            fontSize: '24px',
                            cursor: 'pointer',
                            color: '#999',
                        }}
                    >
                        ✕
                    </button>
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                    <div>
                        <p style={{ fontSize: '12px', color: '#666', margin: 0 }}>Room Name</p>
                        <p style={{ fontWeight: 500, margin: 0 }}>{roomDetails.room_name}</p>
                    </div>
                    <div>
                        <p style={{ fontSize: '12px', color: '#666', margin: 0 }}>Created By</p>
                        <p style={{ fontWeight: 500, margin: 0 }}>{roomDetails.creator_username}</p>
                    </div>
                    <div>
                        <p style={{ fontSize: '12px', color: '#666', margin: 0 }}>Admin</p>
                        <p style={{ fontWeight: 500, margin: 0 }}>{roomDetails.admin_username}</p>
                    </div>
                    <div>
                        <p style={{ fontSize: '12px', color: '#666', margin: 0 }}>Created At</p>
                        <p style={{ fontWeight: 500, margin: 0 }}>
                            {new Date(roomDetails.created_at).toLocaleString()}
                        </p>
                    </div>
                    <div>
                        <p style={{ fontSize: '12px', color: '#666', marginBottom: '8px', margin: 0 }}>
                            Members ({roomDetails.members.length})
                        </p>
                        <ul style={{
                            background: '#f9fafb',
                            borderRadius: '4px',
                            padding: '8px',
                            maxHeight: '192px',
                            overflowY: 'auto',
                            listStyle: 'none',
                            margin: 0,
                            paddingLeft: 0,
                        }}>
                            {roomDetails.members.map((member, idx) => (
                                <li key={idx} style={{
                                    display: 'flex',
                                    justifyContent: 'space-between',
                                    alignItems: 'center',
                                    fontSize: '12px',
                                    padding: '6px 0',
                                }}>
                                    <span>{member.username}</span>
                                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                                        {roomDetails.admin_username === username &&
                                            member.username !== username && (
                                                <button
                                                    onClick={() => {
                                                        if (
                                                            confirm(
                                                                `Are you sure you want to kick ${member.username}?`
                                                            )
                                                        ) {
                                                            send({
                                                                type: 'kick_member',
                                                                room_id: roomDetails.room_id,
                                                                username: member.username,
                                                            });
                                                        }
                                                    }}
                                                    style={{
                                                        color: '#dc2626',
                                                        background: 'none',
                                                        border: 'none',
                                                        cursor: 'pointer',
                                                        fontSize: '11px',
                                                        fontWeight: 'bold',
                                                    }}
                                                >
                                                    Kick
                                                </button>
                                            )}
                                        <span style={{ color: '#999', fontSize: '11px' }}>
                                            {new Date(member.joined_at).toLocaleDateString()}
                                        </span>
                                    </div>
                                </li>
                            ))}
                        </ul>
                    </div>
                </div>
            </div>
        </div>
    );
};
