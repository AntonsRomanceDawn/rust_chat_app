import React from 'react';
import { InvitationInfo } from '../types';

interface InvitationListProps {
    invitations: InvitationInfo[];
    onAccept: (invitationId: string) => void;
    onDecline: (invitationId: string) => void;
}

export const InvitationList: React.FC<InvitationListProps> = ({
    invitations,
    onAccept,
    onDecline,
}) => {
    return (
        <div style={{ flex: 1, overflowY: 'auto', padding: '12px' }}>
            {invitations.length === 0 ? (
                <div style={{ textAlign: 'center', color: '#999', fontSize: '12px' }}>
                    No pending invitations
                </div>
            ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                    {invitations.map((inv) => (
                        <div
                            key={inv.invitation_id}
                            style={{
                                background: '#fff',
                                border: '1px solid #ddd',
                                borderRadius: '6px',
                                padding: '12px',
                                fontSize: '12px',
                            }}
                        >
                            <p style={{ marginBottom: '8px', margin: 0 }}>
                                <span style={{ fontWeight: 'bold', color: '#2563eb' }}>
                                    {inv.inviter_username}
                                </span>{' '}
                                invited you to{' '}
                                <span style={{ fontWeight: 'bold' }}>{inv.room_name}</span>
                            </p>
                            <div style={{ display: 'flex', gap: '8px' }}>
                                <button
                                    onClick={() => onAccept(inv.invitation_id)}
                                    style={{
                                        flex: 1,
                                        background: '#16a34a',
                                        color: 'white',
                                        border: 'none',
                                        padding: '6px',
                                        borderRadius: '4px',
                                        fontSize: '11px',
                                        fontWeight: 'bold',
                                        cursor: 'pointer',
                                    }}
                                >
                                    Accept
                                </button>
                                <button
                                    onClick={() => onDecline(inv.invitation_id)}
                                    style={{
                                        flex: 1,
                                        background: '#dc2626',
                                        color: 'white',
                                        border: 'none',
                                        padding: '6px',
                                        borderRadius: '4px',
                                        fontSize: '11px',
                                        fontWeight: 'bold',
                                        cursor: 'pointer',
                                    }}
                                >
                                    Decline
                                </button>
                            </div>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
};
