# Encrypted Chat App

A real-time chat application built with Rust (Axum) backend and React (Vite) frontend.

## Features

- **End-to-End Encryption (E2EE)**: Secure messaging using the Signal Protocol (X3DH + Double Ratchet). Messages are encrypted on the device and only readable by the intended recipients.
- **Authentication**: Secure user registration and login using JWT (Access & Refresh Tokens).
- **Real-time Messaging**: WebSocket-based communication for instant message delivery.
- **Rooms**: Create and join chat rooms.
- **Invitations**: Invite other users to private rooms.
- **Modern UI**: Responsive web interface built with React and Tailwind CSS.

## Tech Stack

### Backend (`/server`)
- **Language**: Rust
- **Framework**: [Axum](https://github.com/tokio-rs/axum)
- **Database**: PostgreSQL (via [SQLx](https://github.com/launchbadge/sqlx))
- **Async Runtime**: Tokio
- **Security**: Scrypt for password hashing, JWT for sessions.
- **Encryption**: Signal Protocol key management.

### Frontend (`/web_client`)
- **Framework**: React
- **Build Tool**: Vite
- **Styling**: Tailwind CSS
- **HTTP Client**: Axios
- **Encryption**: `@privacyresearch/libsignal-protocol-typescript` (requires `vite-plugin-node-polyfills`)

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) & npm
- [PostgreSQL](https://www.postgresql.org/)
- [SQLx CLI](https://github.com/launchbadge/sqlx#install-the-cli) (`cargo install sqlx-cli`)

## Getting Started

### Quick Setup

We have provided a setup script to install all dependencies and prepare the project.

```bash
chmod +x setup.sh
./setup.sh
```

### Manual Setup

#### 1. Database Setup

Ensure your PostgreSQL server is running and create a database.

```bash
# Create a .env file in the server directory
cd server
cp .env.example .env

# Edit .env and set your DATABASE_URL
# Example: DATABASE_URL=postgres://user:password@localhost/chat_db
```

Run migrations to set up the schema:

```bash
sqlx database create
sqlx migrate run
```

### 2. Run the Server

```bash
cd server
cargo run
```

The server will start on `http://localhost:3000`.

### 3. Run the Web Client

Open a new terminal window:

```bash
cd web_client
npm install
npm run dev
```

The web client will be available at `http://localhost:5173` (or the port shown in your terminal).

## Project Structure

```
.
├── server/           # Rust backend code
│   ├── migrations/   # SQLx database migrations
│   └── src/          # Source code (Handlers, Models, WebSocket logic)
├── web_client/       # React frontend code
│   └── src/          # Components and hooks
└── Cargo.toml        # Workspace configuration
```

## License

This project is licensed under the MIT License.
