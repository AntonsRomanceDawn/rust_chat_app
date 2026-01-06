import React, { useMemo } from 'react';
import { MessageInfo } from '../types';

interface MessageBubbleProps {
    msg: MessageInfo;
    isOwn: boolean;
}

const MessageBubble: React.FC<MessageBubbleProps> = ({ msg, isOwn }) => {
    const time = useMemo(() => new Date(msg.created_at).toLocaleTimeString(), [msg.created_at]);

    return (
        <div style={{ display: 'flex', justifyContent: isOwn ? 'flex-end' : 'flex-start' }}>
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: isOwn ? 'flex-end' : 'flex-start' }}>
                <div
                    style={{
                        maxWidth: '70%',
                        backgroundColor: isOwn ? '#2563eb' : '#fff',
                        color: isOwn ? '#fff' : '#000',
                        border: isOwn ? 'none' : '1px solid #ddd',
                        padding: '12px',
                        borderRadius: '8px',
                        wordWrap: 'break-word',
                    }}
                >
                    <p style={{ fontSize: '11px', opacity: isOwn ? 0.8 : 0.6, margin: '0 0 4px 0' }}>
                        {msg.author_username}
                    </p>
                    <p style={{ margin: 0, whiteSpace: 'pre-wrap' }}>
                        {msg.content}
                    </p>
                </div>
                <span style={{ fontSize: '10px', opacity: 0.5, marginTop: '4px' }}>{time}</span>
            </div>
        </div>
    );
};

interface MessageListProps {
    messages: MessageInfo[];
    currentUsername: string | null;
}

export const MessageList: React.FC<MessageListProps> = ({ messages, currentUsername }) => {
    return (
        <div style={{ flex: 1, overflowY: 'auto', padding: '16px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
            {messages.map((msg, i) => (
                <MessageBubble
                    key={msg.message_id || i}
                    msg={msg}
                    isOwn={msg.author_username === currentUsername}
                />
            ))}
        </div>
    );
};
