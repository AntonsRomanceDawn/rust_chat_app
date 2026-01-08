import React from 'react';

interface NotificationProps {
    message: string;
    type: 'error' | 'success' | 'info';
}

export const Notification: React.FC<NotificationProps> = ({ message, type }) => {
    return (
        <div style={{
            position: 'fixed',
            bottom: '16px',
            left: '16px',
            background: type === 'error' ? '#dc2626' : (type === 'success' ? '#16a34a' : '#2563eb'),
            color: 'white',
            padding: '16px',
            borderRadius: '6px',
            boxShadow: '0 4px 6px rgba(0,0,0,0.1)',
            zIndex: 50,
            maxWidth: '320px',
        }}>
            <p style={{ fontSize: '12px', margin: 0 }}>{message}</p>
        </div>
    );
};
