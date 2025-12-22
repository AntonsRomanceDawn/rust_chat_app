#!/bin/bash
set -e

echo "🚀 Setting up Encrypted Chat App..."

# Check for prerequisites
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust (cargo) is not installed. Please install it first."
    exit 1
fi

if ! command -v npm &> /dev/null; then
    echo "❌ Node.js (npm) is not installed. Please install it first."
    exit 1
fi

if ! command -v sqlx &> /dev/null; then
    echo "⚠️  sqlx-cli is not installed. Installing it now..."
    cargo install sqlx-cli
fi

# Server Setup
echo "📦 Setting up Server..."
cd server
if [ ! -f .env ]; then
    echo "⚠️  No .env file found in server/. Please create one based on .env.example"
else
    echo "🔄 Running Database Migrations..."
    sqlx migrate run
fi
echo "Compiling server..."
cargo build
cd ..

# Client Setup
echo "📦 Setting up Web Client..."
cd web_client
echo "📥 Installing npm dependencies..."
npm install
echo "➕ Installing Signal Protocol dependencies..."
npm install @privacyresearch/libsignal-protocol-typescript buffer
cd ..

echo "✅ Setup Complete!"
echo "To start the app:"
echo "1. Terminal 1: cd server && cargo run"
echo "2. Terminal 2: cd web_client && npm run dev"
