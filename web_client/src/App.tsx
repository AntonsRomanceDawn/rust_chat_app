import { useState, useEffect } from 'react';
import axios from 'axios';
import { useChat } from './useChat';
import { LoginResp, RegisterResp } from './types';
import { AuthScreen } from './components/AuthScreen';
import { Sidebar } from './components/Sidebar';
import { ChatArea } from './components/ChatArea';
import { RoomInfoModal } from './components/RoomInfoModal';
import { Notification } from './components/Notification';

const API_URL = 'http://localhost:3000';

function parseJwt(token: string) {
    try {
        const base64Url = token.split('.')[1];
        const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/');
        const jsonPayload = decodeURIComponent(
            window
                .atob(base64)
                .split('')
                .map((c) => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2))
                .join('')
        );
        return JSON.parse(jsonPayload);
    } catch (e) {
        return null;
    }
}

function App() {
    const [token, setToken] = useState<string | null>(
        localStorage.getItem('token')
    );
    const [username, setUsername] = useState<string | null>(
        localStorage.getItem('username')
    );

    const handleLogout = () => {
        setToken(null);
        setUsername(null);
        localStorage.removeItem('token');
        localStorage.removeItem('refresh_token');
        localStorage.removeItem('username');
        localStorage.removeItem('signal_store_v2');
    };

    const refreshAuthToken = async (): Promise<string | null> => {
        const refreshToken = localStorage.getItem('refresh_token');
        if (!refreshToken) {
            handleLogout();
            return null;
        }
        try {
            const res = await axios.post(`${API_URL}/refresh-token`, {
                refresh_token: refreshToken,
            });
            const newToken = res.data.access_token;
            const newRefreshToken = res.data.refresh_token;
            setToken(newToken);
            localStorage.setItem('token', newToken);
            localStorage.setItem('refresh_token', newRefreshToken);
            return newToken;
        } catch (e) {
            console.error('Refresh failed', e);
            handleLogout();
            return null;
        }
    };

    const {
        rooms,
        currentRoom,
        setCurrentRoom,
        messages,
        invitations,
        searchResults,
        setSearchResults,
        isConnected,
        send,
        clearUnread,
        roomDetails,
        setRoomDetails,
        notification,
        setNotification,
    } = useChat(token, username, refreshAuthToken);

    const [messageInput, setMessageInput] = useState('');
    const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
    const [newRoomName, setNewRoomName] = useState('');
    const [searchQuery, setSearchQuery] = useState('');
    const [showRoomInfo, setShowRoomInfo] = useState(false);
    const [activeTab, setActiveTab] = useState<'rooms' | 'invitations'>('rooms');
    const [usernameInput, setUsernameInput] = useState('');
    const [passwordInput, setPasswordInput] = useState('');
    const [confirmPasswordInput, setConfirmPasswordInput] = useState('');
    const [isLoginMode, setIsLoginMode] = useState(true);

    useEffect(() => {
        if (notification) {
            const timer = setTimeout(() => setNotification(null), 3000);
            return () => clearTimeout(timer);
        }
    }, [notification, setNotification]);

    useEffect(() => {
        if (!token) return;

        const decoded = parseJwt(token);
        if (!decoded || !decoded.exp) return;

        const expiresInMs = decoded.exp * 1000 - Date.now();
        const refreshTime = Math.max(0, expiresInMs - 10000);

        console.log(
            `Token expires in ${expiresInMs / 1000}s. Scheduling refresh in ${refreshTime / 1000}s`
        );

        const timer = setTimeout(() => {
            console.log('Refreshing token proactively...');
            refreshAuthToken();
        }, refreshTime);

        return () => clearTimeout(timer);
    }, [token]);

    const handleAuth = async (e: React.FormEvent) => {
        e.preventDefault();
        try {
            if (isLoginMode) {
                const res = await axios.post<LoginResp>(`${API_URL}/login`, {
                    username: usernameInput,
                    password: passwordInput,
                });
                setToken(res.data.access_token);
                setUsername(usernameInput);
                localStorage.setItem('token', res.data.access_token);
                localStorage.setItem('refresh_token', res.data.refresh_token);
                localStorage.setItem('username', usernameInput);
            } else {
                if (passwordInput !== confirmPasswordInput) {
                    setNotification({
                        message: 'Passwords do not match',
                        type: 'error',
                    });
                    return;
                }
                await axios.post<RegisterResp>(`${API_URL}/register`, {
                    username: usernameInput,
                    password: passwordInput,
                    confirm_password: confirmPasswordInput,
                });
                setNotification({
                    message: 'Registered successfully! Please login.',
                    type: 'success',
                });
                setIsLoginMode(true);
                setUsernameInput('');
                setPasswordInput('');
                setConfirmPasswordInput('');
            }
        } catch (err: any) {
            let message = 'Auth failed';
            if (err.response?.data?.errors?.[0]?.code) {
                message = err.response.data.errors[0].code.replace(/_/g, ' ');
                message = message.charAt(0).toUpperCase() + message.slice(1);
            }
            setNotification({ message, type: 'error' });
        }
    };

    const handleSendMessage = () => {
        if (!currentRoom || !messageInput.trim()) return;

        if (editingMessageId) {
            send({
                type: 'edit_message',
                message_id: editingMessageId,
                new_content: messageInput,
            });
            setEditingMessageId(null);
        } else {
            send({
                type: 'send_message',
                room_id: currentRoom,
                content: messageInput,
            });
        }
        setMessageInput('');
    };

    const startEditing = (msg: any) => {
        setEditingMessageId(msg.message_id);
        setMessageInput(msg.content);
    };

    const handleDeleteMessage = (messageId: string) => {
        if (confirm('Are you sure you want to delete this message?')) {
            send({
                type: 'delete_message',
                message_id: messageId,
            });
        }
    };

    const handleCreateRoom = (e: React.FormEvent) => {
        e.preventDefault();
        if (!newRoomName.trim()) return;
        send({ type: 'create_room', name: newRoomName });
        setNewRoomName('');
    };

    const handleSearchUsers = (e: React.FormEvent) => {
        e.preventDefault();
        if (!searchQuery.trim()) return;
        send({ type: 'search_users', query: searchQuery });
    };

    const handleInvite = (targetUsername: string) => {
        if (!currentRoom) return;
        if (targetUsername === username) {
            setNotification({
                message: 'You cannot invite yourself.',
                type: 'error',
            });
            return;
        }
        send({
            type: 'invite',
            room_id: currentRoom,
            username: targetUsername,
        });
        setSearchQuery('');
        setSearchResults([]);
    };

    const handleFileUpload = async (file: File) => {
        if (!currentRoom || !token) {
            setNotification({
                message: 'Cannot upload file: no room selected or not authenticated',
                type: 'error',
            });
            return;
        }

        try {
            // Read file as ArrayBuffer
            const arrayBuffer = await file.arrayBuffer();
            const fileData = new Uint8Array(arrayBuffer);

            // Create metadata with filename and file type
            const metadata = JSON.stringify({
                filename: file.name,
                mimeType: file.type,
                size: file.size,
            });

            // Encrypt file data and metadata (you'll need to implement encryption in signalManager)
            // For now, we'll send as is - you should add encryption here
            const formData = new FormData();
            formData.append('encrypted_data', new Blob([fileData]));
            formData.append('encrypted_metadata', new Blob([new TextEncoder().encode(metadata)]));

            setNotification({
                message: `Uploading ${file.name}...`,
                type: 'success',
            });

            // Upload file
            const uploadRes = await axios.post(`${API_URL}/files`, formData, {
                headers: {
                    'Authorization': `Bearer ${token}`,
                    'Content-Type': 'multipart/form-data',
                },
            });

            // Send message with file reference
            const fileMessage = JSON.stringify({
                type: 'file',
                file_id: uploadRes.data.file_id,
                filename: file.name,
                size: file.size,
                mimeType: file.type,
            });

            send({
                type: 'send_message',
                room_id: currentRoom,
                content: fileMessage,
            });

            setNotification({
                message: `File ${file.name} uploaded successfully!`,
                type: 'success',
            });
        } catch (err: any) {
            console.error('File upload failed', err);
            setNotification({
                message: 'Failed to upload file',
                type: 'error',
            });
        }
    };

    if (!token) {
        return (
            <AuthScreen
                onAuth={handleAuth}
                isLoginMode={isLoginMode}
                setIsLoginMode={setIsLoginMode}
                usernameInput={usernameInput}
                setUsernameInput={setUsernameInput}
                passwordInput={passwordInput}
                setPasswordInput={setPasswordInput}
                confirmPasswordInput={confirmPasswordInput}
                setConfirmPasswordInput={setConfirmPasswordInput}
                notification={notification}
            />
        );
    }

    const currentRoomObj = rooms.find((r) => r.room_id === currentRoom);
    const currentMessages = (currentRoom && messages[currentRoom]) || [];

    return (
        <div style={{
            height: '100vh',
            width: '100vw',
            background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
            padding: '20px',
            boxSizing: 'border-box',
        }}>
            <div style={{
                height: '100%',
                display: 'flex',
                gap: '20px',
                maxWidth: '1400px',
                margin: '0 auto',
            }}>
                {/* Sidebar */}
                <Sidebar
                    activeTab={activeTab}
                    setActiveTab={setActiveTab}
                    rooms={rooms}
                    invitations={invitations}
                    currentRoom={currentRoom}
                    setCurrentRoom={setCurrentRoom}
                    newRoomName={newRoomName}
                    setNewRoomName={setNewRoomName}
                    handleCreateRoom={handleCreateRoom}
                    clearUnread={clearUnread}
                    send={send}
                    isConnected={isConnected}
                    handleLogout={handleLogout}
                />

                {/* Main Chat Area */}
                {currentRoom ? (
                    <ChatArea
                        roomName={currentRoomObj?.room_name || ''}
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
                        messages={currentMessages}
                        startEditing={startEditing}
                        handleDeleteMessage={handleDeleteMessage}
                        messageInput={messageInput}
                        setMessageInput={setMessageInput}
                        editingMessageId={editingMessageId}
                        setEditingMessageId={setEditingMessageId}
                        handleSendMessage={handleSendMessage}
                        handleFileUpload={handleFileUpload}
                        token={token}
                    />
                ) : (
                    <div style={{
                        flex: 1,
                        background: 'rgba(255, 255, 255, 0.95)',
                        borderRadius: '12px',
                        boxShadow: '0 8px 32px rgba(0, 0, 0, 0.1)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: '#999',
                        fontSize: '18px',
                    }}>
                        Select a room to start chatting
                    </div>
                )}
            </div>

            {/* Room Info Modal */}
            {showRoomInfo && roomDetails && (
                <RoomInfoModal
                    roomDetails={roomDetails}
                    setShowRoomInfo={setShowRoomInfo}
                    setRoomDetails={setRoomDetails}
                    username={username}
                    send={send}
                />
            )}

            {/* Notification */}
            {notification && <Notification message={notification.message} type={notification.type} />}
        </div>
    );
}

export default App;
