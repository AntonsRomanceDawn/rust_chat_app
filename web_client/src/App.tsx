import { useState, useEffect } from 'react';
import axios from 'axios';
import { useChat } from './useChat';
import { LoginResp, RegisterResp } from './types';

const API_URL = 'http://localhost:3000'; // Or https:// if using HTTPS

function parseJwt(token: string) {
    try {
        const base64Url = token.split('.')[1];
        const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/');
        const jsonPayload = decodeURIComponent(window.atob(base64).split('').map(function (c) {
            return '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2);
        }).join(''));
        return JSON.parse(jsonPayload);
    } catch (e) {
        return null;
    }
}

function App() {
    const [token, setToken] = useState<string | null>(localStorage.getItem('token'));
    const [username, setUsername] = useState<string | null>(localStorage.getItem('username'));

    const [authMode, setAuthMode] = useState<'login' | 'register'>('login');
    const [authInput, setAuthInput] = useState({ username: '', password: '', confirmPassword: '' });

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
            const res = await axios.post(`${API_URL}/refresh-token`, { refresh_token: refreshToken });
            const newToken = res.data.access_token;
            const newRefreshToken = res.data.refresh_token;
            setToken(newToken);
            localStorage.setItem('token', newToken);
            localStorage.setItem('refresh_token', newRefreshToken);
            return newToken;
        } catch (e) {
            console.error("Refresh failed", e);
            handleLogout();
            return null;
        }
    };

    const {
        rooms, currentRoom, setCurrentRoom, messages, invitations, searchResults,
        error: wsError, isConnected, send, unreadCounts, clearUnread, roomDetails, setRoomDetails,
        notification, setNotification
    } = useChat(token, username, refreshAuthToken);

    const [messageInput, setMessageInput] = useState('');
    const [newRoomName, setNewRoomName] = useState('');
    const [searchQuery, setSearchQuery] = useState('');
    const [showInvitePopover, setShowInvitePopover] = useState(false);
    const [showRoomInfo, setShowRoomInfo] = useState(false);
    const [activeTab, setActiveTab] = useState<'rooms' | 'invitations'>('rooms');

    useEffect(() => {
        if (notification) {
            const timer = setTimeout(() => setNotification(null), 3000);
            return () => clearTimeout(timer);
        }
    }, [notification, setNotification]);

    // Proactive token refresh
    useEffect(() => {
        if (!token) return;

        const decoded = parseJwt(token);
        if (!decoded || !decoded.exp) return;

        // exp is in seconds, Date.now() in ms
        const expiresInMs = (decoded.exp * 1000) - Date.now();

        // Refresh 10 seconds before expiry, or immediately if already close/expired
        // Ensure we don't refresh if it's WAY in the future (e.g. > 1 day? depends on policy)
        // But for short lived tokens (30s), this is crucial.
        const refreshTime = Math.max(0, expiresInMs - 10000);

        console.log(`Token expires in ${expiresInMs / 1000}s. Scheduling refresh in ${refreshTime / 1000}s`);

        const timer = setTimeout(() => {
            console.log("Refreshing token proactively...");
            refreshAuthToken();
        }, refreshTime);

        return () => clearTimeout(timer);
    }, [token]);

    const handleAuth = async (e: React.FormEvent) => {
        e.preventDefault();
        try {
            if (authMode === 'login') {
                const res = await axios.post<LoginResp>(`${API_URL}/login`, {
                    username: authInput.username,
                    password: authInput.password
                });
                setToken(res.data.access_token);
                setUsername(authInput.username);
                localStorage.setItem('token', res.data.access_token);
                localStorage.setItem('refresh_token', res.data.refresh_token);
                localStorage.setItem('username', authInput.username);
            } else {
                const res = await axios.post<RegisterResp>(`${API_URL}/register`, {
                    username: authInput.username,
                    password: authInput.password,
                    confirm_password: authInput.confirmPassword
                });
                // Auto login after register? Or just switch to login
                setAuthMode('login');
                setNotification({ message: 'Registered successfully! Please login.', type: 'success' });
            }
        } catch (err: any) {
            let message = 'Auth failed';
            if (err.response?.data?.errors?.[0]?.code) {
                message = err.response.data.errors[0].code.replace(/_/g, ' ');
                // Capitalize first letter
                message = message.charAt(0).toUpperCase() + message.slice(1);
            }
            setNotification({ message, type: 'error' });
        }
    };

    const handleSendMessage = (e: React.FormEvent) => {
        e.preventDefault();
        if (!currentRoom || !messageInput.trim()) return;
        send({ type: 'send_message', room_id: currentRoom, content: messageInput });
        setMessageInput('');
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
            setNotification({ message: "You cannot invite yourself.", type: 'error' });
            return;
        }
        send({ type: 'invite', room_id: currentRoom, username: targetUsername });
    };

    if (!token) {
        return (
            <div className="min-h-screen flex items-center justify-center bg-gray-100">
                <div className="bg-white p-8 rounded shadow-md w-96">
                    <h2 className="text-2xl mb-4 font-bold text-center">{authMode === 'login' ? 'Login' : 'Register'}</h2>
                    <form onSubmit={handleAuth} className="space-y-4">
                        <input
                            type="text"
                            placeholder="Username"
                            className="w-full p-2 border rounded"
                            value={authInput.username}
                            onChange={e => setAuthInput({ ...authInput, username: e.target.value })}
                        />
                        <input
                            type="password"
                            placeholder="Password"
                            className="w-full p-2 border rounded"
                            value={authInput.password}
                            onChange={e => setAuthInput({ ...authInput, password: e.target.value })}
                        />
                        {authMode === 'register' && (
                            <input
                                type="password"
                                placeholder="Confirm Password"
                                className="w-full p-2 border rounded"
                                value={authInput.confirmPassword}
                                onChange={e => setAuthInput({ ...authInput, confirmPassword: e.target.value })}
                            />
                        )}
                        <button type="submit" className="w-full bg-blue-500 text-white p-2 rounded hover:bg-blue-600">
                            {authMode === 'login' ? 'Login' : 'Register'}
                        </button>
                    </form>
                    <p className="mt-4 text-center text-sm text-blue-500 cursor-pointer" onClick={() => setAuthMode(authMode === 'login' ? 'register' : 'login')}>
                        {authMode === 'login' ? 'Need an account? Register' : 'Have an account? Login'}
                    </p>
                </div>
                {notification && (
                    <div className={`fixed bottom-4 right-4 px-4 py-2 rounded shadow-lg z-50 text-white ${notification.type === 'error' ? 'bg-red-500' : 'bg-green-500'}`}>
                        {notification.message}
                    </div>
                )}
            </div>
        );
    }

    return (
        <div className="h-screen flex flex-col bg-gray-50 overflow-hidden">
            {/* Header */}
            <header className="bg-white shadow p-4 flex justify-between items-center flex-shrink-0 z-10">
                <h1 className="text-xl font-bold">Encrypted Chat</h1>
                <div className="flex items-center gap-4">
                    <span>Welcome, {username}</span>
                    <div className={`w-3 h-3 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} title={isConnected ? 'Connected' : 'Disconnected'}></div>
                    <button onClick={handleLogout} className="text-red-500 hover:text-red-700">Logout</button>
                </div>
            </header>

            <div className="flex flex-1 overflow-hidden">
                {/* Sidebar */}
                <aside className="w-64 bg-white border-r flex flex-col">
                    {/* Tabs */}
                    <div className="flex border-b">
                        <button
                            className={`flex-1 py-3 text-sm font-medium text-center ${activeTab === 'rooms' ? 'text-blue-600 border-b-2 border-blue-600' : 'text-gray-500 hover:text-gray-700'}`}
                            onClick={() => setActiveTab('rooms')}
                        >
                            Rooms
                        </button>
                        <button
                            className={`flex-1 py-3 text-sm font-medium text-center relative ${activeTab === 'invitations' ? 'text-blue-600 border-b-2 border-blue-600' : 'text-gray-500 hover:text-gray-700'}`}
                            onClick={() => setActiveTab('invitations')}
                        >
                            Invitations
                            {invitations.length > 0 && (
                                <span className="absolute top-2 right-2 bg-red-500 text-white text-[10px] font-bold px-1.5 py-0.5 rounded-full">
                                    {invitations.length}
                                </span>
                            )}
                        </button>
                    </div>

                    <div className="flex-1 overflow-hidden flex flex-col">
                        {activeTab === 'rooms' ? (
                            <>
                                <div className="p-4 border-b bg-gray-50">
                                    <h3 className="font-semibold mb-2 text-xs uppercase text-gray-500">Create Room</h3>
                                    <form onSubmit={handleCreateRoom} className="flex gap-2">
                                        <input
                                            type="text"
                                            placeholder="Room Name"
                                            className="flex-1 p-2 border rounded text-sm focus:outline-none focus:border-blue-500"
                                            value={newRoomName}
                                            onChange={e => setNewRoomName(e.target.value)}
                                        />
                                        <button type="submit" className="bg-blue-500 text-white w-10 rounded hover:bg-blue-600 transition-colors flex items-center justify-center text-xl font-bold pb-1">+</button>
                                    </form>
                                </div>
                                <div className="flex-1 overflow-y-auto p-2">
                                    {rooms.length === 0 ? (
                                        <p className="text-center text-gray-400 text-sm mt-4">No rooms yet.</p>
                                    ) : (
                                        <ul className="space-y-1">
                                            {rooms.map(room => (
                                                <li
                                                    key={room.room_id}
                                                    onClick={() => {
                                                        setCurrentRoom(room.room_id);
                                                        clearUnread(room.room_id);
                                                        send({ type: 'get_messages', room_id: room.room_id, limit: 50, offset: 0 });
                                                    }}
                                                    className={`p-2 rounded cursor-pointer flex justify-between items-center transition-colors ${currentRoom === room.room_id ? 'bg-blue-100 text-blue-700' : 'hover:bg-gray-100'}`}
                                                >
                                                    <span className="truncate font-medium"># {room.room_name}</span>
                                                    {unreadCounts[room.room_id] > 0 && (
                                                        <span className="bg-red-500 text-white text-xs font-bold px-2 py-0.5 rounded-full">
                                                            {unreadCounts[room.room_id]}
                                                        </span>
                                                    )}
                                                </li>
                                            ))}
                                        </ul>
                                    )}
                                </div>
                            </>
                        ) : (
                            <div className="flex-1 overflow-y-auto p-4">
                                {invitations.length === 0 ? (
                                    <p className="text-center text-gray-400 text-sm mt-4">No pending invitations.</p>
                                ) : (
                                    <ul className="space-y-3">
                                        {invitations.map(inv => (
                                            <li key={inv.invitation_id} className="p-3 bg-white border border-gray-200 rounded shadow-sm text-sm">
                                                <p className="mb-2">
                                                    <span className="font-bold text-blue-600">{inv.inviter_username}</span> invited you to <span className="font-bold">{inv.room_name}</span>
                                                </p>
                                                <div className="flex gap-2">
                                                    <button
                                                        onClick={() => send({ type: 'join_room', invitation_id: inv.invitation_id })}
                                                        className="flex-1 bg-green-500 text-white py-1.5 rounded text-xs font-medium hover:bg-green-600 transition-colors"
                                                    >
                                                        Accept
                                                    </button>
                                                    <button
                                                        onClick={() => send({ type: 'decline_invitation', invitation_id: inv.invitation_id })}
                                                        className="flex-1 bg-red-500 text-white py-1.5 rounded text-xs font-medium hover:bg-red-600 transition-colors"
                                                    >
                                                        Decline
                                                    </button>
                                                </div>
                                            </li>
                                        ))}
                                    </ul>
                                )}
                            </div>
                        )}
                    </div>
                </aside>

                {/* Main Chat Area */}
                <main className="flex-1 flex flex-col min-w-0">
                    {currentRoom ? (
                        <>
                            <div className="p-4 border-b bg-white flex justify-between items-center flex-shrink-0">
                                <div className="flex items-center gap-2">
                                    <h2 className="font-bold text-lg"># {rooms.find(r => r.room_id === currentRoom)?.room_name}</h2>
                                    <button
                                        onClick={() => {
                                            if (currentRoom) {
                                                send({ type: 'get_room_info', room_id: currentRoom });
                                                setShowRoomInfo(true);
                                            }
                                        }}
                                        className="text-gray-500 hover:text-gray-700"
                                        title="Room Info"
                                    >
                                        ℹ️
                                    </button>
                                </div>
                                <div className="flex gap-2">
                                    {/* User Search / Invite Popover */}
                                    <div className="relative">
                                        <button
                                            className="text-sm text-blue-500 hover:underline"
                                            onClick={() => setShowInvitePopover(!showInvitePopover)}
                                        >
                                            Invite Users
                                        </button>
                                        {showInvitePopover && (
                                            <div className="absolute right-0 top-full mt-2 w-64 bg-white border shadow-lg rounded p-2 z-10">
                                                <form onSubmit={handleSearchUsers} className="flex gap-2 mb-2">
                                                    <input
                                                        className="flex-1 border p-1 text-sm"
                                                        placeholder="Search user..."
                                                        value={searchQuery}
                                                        onChange={e => setSearchQuery(e.target.value)}
                                                    />
                                                    <button className="bg-blue-500 text-white px-2 text-sm">Go</button>
                                                </form>
                                                <ul className="max-h-40 overflow-y-auto">
                                                    {searchResults.filter(u => u.username !== username).map(u => (
                                                        <li key={u.username} className="flex justify-between items-center p-1 hover:bg-gray-50 text-sm">
                                                            <span>{u.username}</span>
                                                            <button onClick={() => handleInvite(u.username)} className="text-blue-500 text-xs">Invite</button>
                                                        </li>
                                                    ))}
                                                </ul>
                                                <button
                                                    onClick={() => setShowInvitePopover(false)}
                                                    className="w-full text-center text-xs text-gray-400 mt-2 hover:text-gray-600"
                                                >
                                                    Close
                                                </button>
                                            </div>
                                        )}
                                    </div>
                                    <button
                                        onClick={() => {
                                            if (confirm('Are you sure you want to leave?')) {
                                                send({ type: 'leave_room', room_id: currentRoom });
                                            }
                                        }}
                                        className="text-red-500 text-sm"
                                    >
                                        Leave Room
                                    </button>
                                </div>
                            </div>

                            <div className="flex-1 overflow-y-auto p-4 space-y-4 bg-gray-50 min-h-0">
                                {messages[currentRoom]?.map((msg, i) => (
                                    <div key={msg.message_id || i} className={`flex flex-col ${msg.author_username === 'Me' || msg.author_username === username ? 'items-end' : 'items-start'}`}>
                                        <div className={`max-w-[70%] rounded-lg p-3 break-words ${msg.author_username === 'Me' || msg.author_username === username ? 'bg-blue-500 text-white' : 'bg-white border'}`}>
                                            <p className="text-xs opacity-75 mb-1">{msg.author_username}</p>
                                            <p className="break-words whitespace-pre-wrap">{msg.content}</p>
                                        </div>
                                        <span className="text-xs text-gray-400 mt-1">{new Date(msg.created_at).toLocaleTimeString()}</span>
                                    </div>
                                ))}
                            </div>

                            <div className="p-4 bg-white border-t flex-shrink-0">
                                <form onSubmit={handleSendMessage} className="flex gap-2">
                                    <input
                                        type="text"
                                        className="flex-1 border rounded p-2"
                                        placeholder="Type a message..."
                                        value={messageInput}
                                        onChange={e => setMessageInput(e.target.value)}
                                    />
                                    <button type="submit" className="bg-blue-500 text-white px-6 rounded hover:bg-blue-600 flex-shrink-0">Send</button>
                                </form>
                            </div>
                        </>
                    ) : (
                        <div className="flex-1 flex items-center justify-center text-gray-400">
                            Select a room to start chatting
                        </div>
                    )}
                </main>
            </div>

            {wsError && (
                <div className="fixed bottom-4 right-4 bg-red-500 text-white p-3 rounded shadow-lg">
                    Error: {wsError}
                    <button onClick={() => window.location.reload()} className="ml-2 underline">Retry</button>
                </div>
            )}

            {/* Room Info Modal */}
            {showRoomInfo && roomDetails && (
                <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div className="bg-white p-6 rounded-lg shadow-xl w-96 max-h-[80vh] overflow-y-auto">
                        <div className="flex justify-between items-center mb-4">
                            <h2 className="text-xl font-bold">Room Info</h2>
                            <button onClick={() => { setShowRoomInfo(false); setRoomDetails(null); }} className="text-gray-500 hover:text-gray-700">✕</button>
                        </div>
                        <div className="space-y-4">
                            <div>
                                <p className="text-sm text-gray-500">Room Name</p>
                                <p className="font-medium">{roomDetails.room_name}</p>
                            </div>
                            <div>
                                <p className="text-sm text-gray-500">Created By</p>
                                <p className="font-medium">{roomDetails.creator_username}</p>
                            </div>
                            <div>
                                <p className="text-sm text-gray-500">Admin</p>
                                <p className="font-medium">{roomDetails.admin_username}</p>
                            </div>
                            <div>
                                <p className="text-sm text-gray-500">Created At</p>
                                <p className="font-medium">{new Date(roomDetails.created_at).toLocaleString()}</p>
                            </div>
                            <div>
                                <p className="text-sm text-gray-500 mb-2">Members ({roomDetails.members.length})</p>
                                <ul className="bg-gray-50 rounded p-2 space-y-2">
                                    {roomDetails.members.map((member, idx) => (
                                        <li key={idx} className="flex justify-between items-center text-sm">
                                            <span>{member.username}</span>
                                            <div className="flex items-center gap-2">
                                                <span className="text-gray-400 text-xs">{new Date(member.joined_at).toLocaleDateString()}</span>
                                                {roomDetails.admin_username === username && member.username !== username && (
                                                    <button
                                                        onClick={() => {
                                                            if (confirm(`Are you sure you want to kick ${member.username}?`)) {
                                                                send({ type: 'kick_member', room_id: roomDetails.room_id, username: member.username });
                                                            }
                                                        }}
                                                        className="text-red-500 hover:text-red-700 text-xs font-bold ml-2"
                                                        title="Kick Member"
                                                    >
                                                        Kick
                                                    </button>
                                                )}
                                            </div>
                                        </li>
                                    ))}
                                </ul>
                            </div>
                        </div>
                    </div>
                </div>
            )}
            {notification && (
                <div className={`fixed bottom-4 right-4 px-4 py-2 rounded shadow-lg z-50 text-white ${notification.type === 'error' ? 'bg-red-500' : 'bg-green-500'}`}>
                    {notification.message}
                </div>
            )}
        </div>
    );
}

export default App;
