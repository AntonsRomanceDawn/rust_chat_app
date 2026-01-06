import React from 'react';

interface Notification {
    message: string;
    type: 'error' | 'success';
}

interface AuthScreenProps {
    onAuth: (e: React.FormEvent) => void;
    isLoginMode: boolean;
    setIsLoginMode: (isLogin: boolean) => void;
    usernameInput: string;
    setUsernameInput: (username: string) => void;
    passwordInput: string;
    setPasswordInput: (password: string) => void;
    confirmPasswordInput: string;
    setConfirmPasswordInput: (password: string) => void;
    notification: Notification | null;
}

export const AuthScreen: React.FC<AuthScreenProps> = ({
    onAuth,
    isLoginMode,
    setIsLoginMode,
    usernameInput,
    setUsernameInput,
    passwordInput,
    setPasswordInput,
    confirmPasswordInput,
    setConfirmPasswordInput,
    notification,
}) => {
    return (
        <div style={{
            width: '100vw',
            height: '100vh',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        }}>
            <div style={{
                background: 'white',
                padding: '40px',
                borderRadius: '12px',
                boxShadow: '0 20px 60px rgba(0, 0, 0, 0.3)',
                width: '100%',
                maxWidth: '400px',
            }}>
                <h1 style={{
                    fontSize: '28px',
                    fontWeight: 'bold',
                    marginBottom: '24px',
                    textAlign: 'center',
                    color: '#1f2937',
                }}>
                    Encrypted Chat
                </h1>

                <form onSubmit={onAuth} style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '16px',
                }}>
                    <div>
                        <label style={{
                            display: 'block',
                            fontSize: '12px',
                            fontWeight: '500',
                            marginBottom: '6px',
                            color: '#374151',
                        }}>
                            Username
                        </label>
                        <input
                            type="text"
                            value={usernameInput}
                            onChange={(e) => setUsernameInput(e.target.value)}
                            required
                            style={{
                                width: '100%',
                                padding: '10px 12px',
                                border: '1px solid #d1d5db',
                                borderRadius: '6px',
                                fontSize: '14px',
                                boxSizing: 'border-box',
                            }}
                            placeholder="Choose a username"
                        />
                    </div>

                    <div>
                        <label style={{
                            display: 'block',
                            fontSize: '12px',
                            fontWeight: '500',
                            marginBottom: '6px',
                            color: '#374151',
                        }}>
                            Password
                        </label>
                        <input
                            type="password"
                            value={passwordInput}
                            onChange={(e) => setPasswordInput(e.target.value)}
                            required
                            style={{
                                width: '100%',
                                padding: '10px 12px',
                                border: '1px solid #d1d5db',
                                borderRadius: '6px',
                                fontSize: '14px',
                                boxSizing: 'border-box',
                            }}
                            placeholder="••••••••"
                        />
                    </div>

                    {!isLoginMode && (
                        <div>
                            <label style={{
                                display: 'block',
                                fontSize: '12px',
                                fontWeight: '500',
                                marginBottom: '6px',
                                color: '#374151',
                            }}>
                                Confirm Password
                            </label>
                            <input
                                type="password"
                                value={confirmPasswordInput}
                                onChange={(e) => setConfirmPasswordInput(e.target.value)}
                                required={!isLoginMode}
                                style={{
                                    width: '100%',
                                    padding: '10px 12px',
                                    border: '1px solid #d1d5db',
                                    borderRadius: '6px',
                                    fontSize: '14px',
                                    boxSizing: 'border-box',
                                }}
                                placeholder="••••••••"
                            />
                        </div>
                    )}

                    {notification && (
                        <div style={{
                            background: notification.type === 'error' ? '#fee2e2' : '#dcfce7',
                            color: notification.type === 'error' ? '#dc2626' : '#15803d',
                            padding: '10px 12px',
                            borderRadius: '6px',
                            fontSize: '12px',
                        }}>
                            {notification.message}
                        </div>
                    )}

                    <button
                        type="submit"
                        style={{
                            background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                            color: 'white',
                            padding: '12px',
                            borderRadius: '6px',
                            border: 'none',
                            fontSize: '14px',
                            fontWeight: '600',
                            cursor: 'pointer',
                        }}
                    >
                        {isLoginMode ? 'Login' : 'Register'}
                    </button>
                </form>

                <div style={{
                    marginTop: '20px',
                    textAlign: 'center',
                }}>
                    <p style={{
                        fontSize: '14px',
                        color: '#6b7280',
                        margin: 0,
                    }}>
                        {isLoginMode ? "Don't have an account?" : 'Already have an account?'}
                        {' '}
                        <button
                            type="button"
                            onClick={() => setIsLoginMode(!isLoginMode)}
                            style={{
                                background: 'none',
                                border: 'none',
                                color: '#667eea',
                                cursor: 'pointer',
                                fontWeight: '600',
                                fontSize: '14px',
                            }}
                        >
                            {isLoginMode ? 'Register' : 'Login'}
                        </button>
                    </p>
                </div>
            </div>
        </div>
    );
};
