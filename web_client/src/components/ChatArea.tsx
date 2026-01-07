import React from 'react';
import { ChatHeader } from './ChatHeader';
import { MessageArea } from './MessageArea';
import { MessageInput } from './MessageInput';

interface Message {
    message_id: string;
    author_username?: string;
    content: string;
    message_type: string;
    message_status: string;
    created_at: string;
}

interface UserInfo {
    username: string;
    created_at: string;
}

interface ChatAreaProps {
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
    messages: Message[];
    startEditing: (msg: Message) => void;
    handleDeleteMessage: (messageId: string) => void;
    messageInput: string;
    setMessageInput: (value: string) => void;
    editingMessageId: string | null;
    setEditingMessageId: (id: string | null) => void;
    handleSendMessage: () => void;
    handleFileUpload: (file: File) => void;
    token: string | null;
}

export const ChatArea: React.FC<ChatAreaProps> = ({
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
    messages,
    startEditing,
    handleDeleteMessage,
    messageInput,
    setMessageInput,
    editingMessageId,
    setEditingMessageId,
    handleSendMessage,
    handleFileUpload,
    token,
}) => {
    return (
        <div style={{
            flex: 1,
            background: 'rgba(255, 255, 255, 0.95)',
            borderRadius: '12px',
            boxShadow: '0 8px 32px rgba(0, 0, 0, 0.1)',
            display: 'flex',
            flexDirection: 'column',
            overflow: 'hidden',
            height: '100%', // Ensure it takes full height available in parent flex
        }}>
            <ChatHeader
                roomName={roomName}
                searchQuery={searchQuery}
                setSearchQuery={setSearchQuery}
                searchResults={searchResults}
                setSearchResults={setSearchResults}
                handleSearchUsers={handleSearchUsers}
                handleInvite={handleInvite}
                username={username}
                send={send}
                currentRoom={currentRoom}
                setShowRoomInfo={setShowRoomInfo}
            />
            <MessageArea
                messages={messages}
                username={username}
                startEditing={startEditing}
                handleDeleteMessage={handleDeleteMessage}
                token={token}
            />
            <MessageInput
                messageInput={messageInput}
                setMessageInput={setMessageInput}
                editingMessageId={editingMessageId}
                setEditingMessageId={setEditingMessageId}
                handleSendMessage={handleSendMessage}
                handleFileUpload={handleFileUpload}
            />
        </div>
    );
};
