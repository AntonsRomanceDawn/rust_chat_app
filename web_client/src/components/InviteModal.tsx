import React from 'react';
import { UserInfo } from '../types';

interface InviteModalProps {
    isOpen: boolean;
    onClose: () => void;
    searchResults: UserInfo[];
    currentUsername: string | null;
    searchQuery: string;
    onSearchChange: (query: string) => void;
    onSearch: (e: React.FormEvent) => void;
    onInvite: (username: string) => void;
}

export const InviteModal: React.FC<InviteModalProps> = ({
    isOpen,
    onClose,
    searchResults,
    currentUsername,
    searchQuery,
    onSearchChange,
    onSearch,
    onInvite,
}) => {
    if (!isOpen) return null;

    return (
        <div style={{
            position: 'absolute',
            bottom: '80px',
            right: '20px',
            background: '#fff',
            border: '1px solid #ccc',
            borderRadius: '6px',
            boxShadow: '0 4px 6px rgba(0,0,0,0.1)',
            padding: '12px',
            width: '260px',
            zIndex: 10,
        }}>
            <h3 style={{ fontWeight: 'bold', marginBottom: '8px', fontSize: '13px', margin: 0 }}>Invite Users</h3>
            <form onSubmit={onSearch} style={{ display: 'flex', gap: '8px', marginBottom: '8px' }}>
                <input
                    style={{
                        flex: 1,
                        border: '1px solid #ccc',
                        padding: '6px',
                        fontSize: '12px',
                        borderRadius: '4px',
                    }}
                    placeholder="Search user..."
                    value={searchQuery}
                    onChange={(e) => onSearchChange(e.target.value)}
                />
                <button style={{
                    background: '#2563eb',
                    color: 'white',
                    border: 'none',
                    padding: '6px 10px',
                    fontSize: '12px',
                    borderRadius: '4px',
                    cursor: 'pointer',
                }}>
                    Search
                </button>
            </form>
            <div style={{ maxHeight: '160px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '4px' }}>
                {searchResults
                    .filter((u) => u.username !== currentUsername)
                    .map((u) => (
                        <div
                            key={u.username}
                            style={{
                                display: 'flex',
                                justifyContent: 'space-between',
                                alignItems: 'center',
                                padding: '6px',
                                background: '#f9fafb',
                                borderRadius: '4px',
                                fontSize: '12px',
                            }}
                        >
                            <span>{u.username}</span>
                            <button
                                onClick={() => onInvite(u.username)}
                                style={{
                                    color: '#2563eb',
                                    background: 'none',
                                    border: 'none',
                                    cursor: 'pointer',
                                    fontSize: '11px',
                                    fontWeight: 'bold',
                                }}
                            >
                                Invite
                            </button>
                        </div>
                    ))}
            </div>
            <button
                onClick={onClose}
                style={{
                    width: '100%',
                    textAlign: 'center',
                    fontSize: '11px',
                    color: '#999',
                    background: 'none',
                    border: 'none',
                    marginTop: '8px',
                    cursor: 'pointer',
                }}
            >
                Close
            </button>
        </div>
    );
};
