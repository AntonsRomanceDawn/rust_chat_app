-- Add up migration script here
-- users
CREATE TABLE users (
    id           UUID PRIMARY KEY,
    username     TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role         TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- rooms
CREATE TABLE rooms (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    creator_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    creator_username TEXT NOT NULL,
    admin_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    admin_username TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- room_members
CREATE TABLE room_members (
    room_id   UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    room_name TEXT NOT NULL,
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    username  TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_read_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    unread_count INT NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, user_id)
);

-- user_messages
CREATE TABLE user_messages (
    id         UUID PRIMARY KEY,
    room_id    UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    room_name  TEXT NOT NULL,
    author_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    author_username TEXT NOT NULL,
    content    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- refresh_tokens
CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- room_invitations
CREATE TABLE invitations (
    id          UUID PRIMARY KEY,
    room_id     UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    room_name   TEXT NOT NULL,
    invitee_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invitee_username TEXT NOT NULL,
    inviter_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    inviter_username TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX unique_pending_invitation ON invitations (room_id, invitee_id, inviter_id) WHERE status = 'pending';
