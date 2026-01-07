import React, { useRef, useEffect, useMemo } from 'react';

const API_URL = import.meta.env.VITE_API_URL ?? '/api';

interface Message {
    message_id: string;
    author_username?: string;
    content: string;
    message_type: string;
    message_status: string;
    created_at: string;
}

interface MessageAreaProps {
    messages: Message[];
    username: string | null;
    startEditing: (msg: Message) => void;
    handleDeleteMessage: (messageId: string) => void;
    token: string | null;
}

interface SystemMessageProps {
    content: string;
}

const SystemMessage: React.FC<SystemMessageProps> = ({ content }) => {
    const text = useMemo(() => {
        try {
            const data = typeof content === 'string' ? JSON.parse(content) : content;
            const bgColor = '#64748b'; // Default Slate-500 for all system messages

            switch (data.type) {
                case 'joined':
                    return { text: `${data.username} joined the room`, color: bgColor };
                case 'left':
                    return { text: `${data.username} left the room`, color: bgColor };
                case 'kicked':
                    return { text: `${data.username} was kicked by ${data.by}`, color: bgColor };
                default:
                    return { text: JSON.stringify(data), color: bgColor };
            }
        } catch (e) {
            return { text: content, color: '#64748b' };
        }
    }, [content]);

    return (
        <div style={{ display: 'flex', justifyContent: 'center', margin: '16px 0', width: '100%' }}>
            <span style={{
                fontSize: '11px',
                color: '#fff',
                backgroundColor: text.color,
                padding: '6px 12px',
                borderRadius: '4px', // More boxy
                fontWeight: 600,
                boxShadow: '0 1px 2px rgba(0,0,0,0.1)',
                textTransform: 'uppercase', // Different form/style
                letterSpacing: '0.5px'
            }}>
                {text.text}
            </span>
        </div>
    );
};

export const MessageArea: React.FC<MessageAreaProps> = ({
    messages,
    username,
    startEditing,
    handleDeleteMessage,
    token,
}) => {
    const messagesEndRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages]);

    const parseFileContent = (content: string): { type: 'file'; file_id: string; filename: string; size: number; mimeType: string } | null => {
        try {
            const parsed = JSON.parse(content);
            if (parsed.type === 'file') {
                return parsed;
            }
        } catch {
            // Not a file message
        }
        return null;
    };

    const formatFileSize = (bytes: number): string => {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
    };

    const handleDownloadFile = async (fileId: string, filename: string, messageId: string) => {
        if (!token) return;
        try {
            console.log('Downloading file:', { fileId, messageId, filename });
            const response = await fetch(`${API_URL}/files/download`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${token}`,
                },
                body: JSON.stringify({
                    file_id: fileId,
                    message_id: messageId,
                }),
            });

            if (!response.ok) {
                const errorText = await response.text();
                console.error('Failed to download file:', response.status, response.statusText, errorText);
                return;
            }

            const data = await response.json();

            // Decrypt the encrypted_data (you might want to use signalManager here for proper decryption)
            const encryptedData = new Uint8Array(Buffer.from(data.encrypted_data, 'utf-8'));

            // Create blob and download
            const blob = new Blob([encryptedData], { type: data.mimeType || 'application/octet-stream' });
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = filename;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            window.URL.revokeObjectURL(url);
        } catch (error) {
            console.error('Error downloading file:', error);
        }
    };

    return (
        <div style={{
            flex: 1,
            overflowY: 'auto',
            padding: '20px',
            display: 'flex',
            flexDirection: 'column',
            gap: '12px',
        }}>
            {messages.map((msg, i) => {
                if (msg.message_type === 'system') {
                    return <SystemMessage key={msg.message_id || i} content={msg.content} />;
                }

                const isOwn = msg.author_username === username;
                const isDeleted = msg.message_status === 'deleted';
                const fileData = parseFileContent(msg.content);
                const isFileMessage = fileData !== null;

                return (
                    <div
                        key={msg.message_id || i}
                        style={{
                            display: 'flex',
                            justifyContent: isOwn ? 'flex-end' : 'flex-start',
                            marginBottom: '4px',
                        }}
                    >
                        <div style={{
                            maxWidth: '70%',
                            display: 'flex',
                            flexDirection: 'column',
                            gap: '4px',
                        }}>
                            {!isOwn && (
                                <span style={{
                                    fontSize: '12px',
                                    fontWeight: 600,
                                    color: '#667eea',
                                    paddingLeft: '12px',
                                }}>
                                    {msg.author_username}
                                </span>
                            )}
                            {isFileMessage && fileData ? (
                                <div style={{
                                    background: isOwn
                                        ? 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
                                        : '#e2e8f0',
                                    color: isOwn ? 'white' : '#1f2937',
                                    padding: '12px 14px',
                                    borderRadius: '12px',
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: '12px',
                                }}>
                                    <span style={{ fontSize: '24px' }}>📁</span>
                                    <div style={{ flex: 1 }}>
                                        <div style={{ fontWeight: 600, fontSize: '13px', marginBottom: '4px', wordBreak: 'break-word' }}>
                                            {fileData.filename}
                                        </div>
                                        <div style={{ fontSize: '11px', opacity: 0.8 }}>
                                            {formatFileSize(fileData.size)}
                                        </div>
                                    </div>
                                    <button
                                        onClick={() => handleDownloadFile(fileData.file_id, fileData.filename, msg.message_id)}
                                        style={{
                                            background: isOwn ? 'rgba(255,255,255,0.2)' : 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                                            color: isOwn ? 'white' : 'white',
                                            border: 'none',
                                            padding: '6px 12px',
                                            borderRadius: '6px',
                                            fontSize: '12px',
                                            fontWeight: 600,
                                            cursor: 'pointer',
                                            whiteSpace: 'nowrap',
                                        }}
                                    >
                                        Download
                                    </button>
                                </div>
                            ) : (
                                <div style={{
                                    background: isOwn
                                        ? 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
                                        : '#e2e8f0',
                                    color: isOwn ? 'white' : '#1f2937',
                                    padding: '10px 14px',
                                    borderRadius: '12px',
                                    fontSize: '14px',
                                    wordBreak: 'break-word',
                                    fontStyle: isDeleted ? 'italic' : 'normal',
                                    opacity: isDeleted ? 0.6 : 1,
                                }}>
                                    {isDeleted ? 'This message was deleted' : msg.content}
                                </div>
                            )}
                            <div style={{
                                display: 'flex',
                                gap: '8px',
                                fontSize: '11px',
                                color: '#9ca3af',
                                paddingLeft: isOwn ? 0 : '12px',
                                paddingRight: isOwn ? '12px' : 0,
                                justifyContent: isOwn ? 'flex-end' : 'flex-start',
                                alignItems: 'center',
                            }}>
                                <span>{new Date(msg.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                                {msg.message_status === 'edited' && <span>(edited)</span>}
                                {isOwn && !isDeleted && (
                                    <>
                                        <span>•</span>
                                        {!isFileMessage && (
                                            <>
                                                <span
                                                    onClick={() => startEditing(msg)}
                                                    style={{
                                                        cursor: 'pointer',
                                                        color: '#667eea',
                                                        fontWeight: 500,
                                                    }}
                                                >
                                                    Edit
                                                </span>
                                                <span>•</span>
                                            </>
                                        )}
                                        <span
                                            onClick={() => handleDeleteMessage(msg.message_id)}
                                            style={{
                                                cursor: 'pointer',
                                                color: '#ef4444',
                                                fontWeight: 500,
                                            }}
                                        >
                                            Delete
                                        </span>
                                    </>
                                )}
                            </div>
                        </div>
                    </div>
                );
            })}
            <div ref={messagesEndRef} />
        </div>
    );
};
