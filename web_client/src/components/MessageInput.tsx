import React, { useRef } from 'react';

interface MessageInputProps {
    messageInput: string;
    setMessageInput: (value: string) => void;
    editingMessageId: string | null;
    setEditingMessageId: (id: string | null) => void;
    handleSendMessage: () => void;
    handleFileUpload: (file: File) => void;
}

export const MessageInput: React.FC<MessageInputProps> = ({
    messageInput,
    setMessageInput,
    editingMessageId,
    setEditingMessageId,
    handleSendMessage,
    handleFileUpload,
}) => {
    const fileInputRef = useRef<HTMLInputElement>(null);

    const onFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (file) {
            handleFileUpload(file);
            // Reset input so same file can be selected again
            if (fileInputRef.current) {
                fileInputRef.current.value = '';
            }
        }
    };

    return (
        <div style={{ borderTop: '1px solid #e5e7eb' }}>
            {editingMessageId && (
                <div style={{
                    padding: '10px 20px',
                    background: '#fef3c7',
                    fontSize: '13px',
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    color: '#92400e',
                    fontWeight: 500,
                }}>
                    <span>✏️ Editing message...</span>
                    <button
                        onClick={() => {
                            setEditingMessageId(null);
                            setMessageInput('');
                        }}
                        style={{
                            border: 'none',
                            background: 'none',
                            cursor: 'pointer',
                            fontSize: '18px',
                            color: '#92400e',
                            padding: 0,
                        }}
                    >
                        ✕
                    </button>
                </div>
            )}
            <div style={{ padding: '16px 20px', display: 'flex', gap: '10px', alignItems: 'center' }}>
                <input
                    ref={fileInputRef}
                    type="file"
                    onChange={onFileSelect}
                    style={{ display: 'none' }}
                    accept="*/*"
                />
                <button
                    onClick={() => fileInputRef.current?.click()}
                    disabled={!!editingMessageId}
                    style={{
                        background: editingMessageId ? '#d1d5db' : 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                        color: 'white',
                        border: 'none',
                        padding: '12px',
                        borderRadius: '8px',
                        fontSize: '18px',
                        cursor: editingMessageId ? 'not-allowed' : 'pointer',
                        transition: 'all 0.2s',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                    }}
                    title="Attach file"
                >
                    📎
                </button>
                <input
                    type="text"
                    placeholder="Type a message..."
                    value={messageInput}
                    onChange={(e) => setMessageInput(e.target.value)}
                    onKeyDown={(e) => {
                        if (e.key === 'Enter' && !e.shiftKey) {
                            e.preventDefault();
                            handleSendMessage();
                        }
                    }}
                    style={{
                        flex: 1,
                        padding: '12px 16px',
                        border: '1px solid #d1d5db',
                        borderRadius: '8px',
                        fontSize: '14px',
                        outline: 'none',
                        transition: 'border 0.2s',
                    }}
                />
                <button
                    onClick={handleSendMessage}
                    disabled={!messageInput.trim()}
                    style={{
                        background: messageInput.trim()
                            ? 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
                            : '#d1d5db',
                        color: 'white',
                        border: 'none',
                        padding: '12px 24px',
                        borderRadius: '8px',
                        fontSize: '14px',
                        fontWeight: 600,
                        cursor: messageInput.trim() ? 'pointer' : 'not-allowed',
                        transition: 'all 0.2s',
                    }}
                >
                    Send
                </button>
            </div>
        </div>
    );
};
