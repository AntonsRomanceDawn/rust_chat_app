# Author's notes
**Motivation:** This project started as a hobby to deepen my Rust skills and explore applied cryptography, with a focus on building secure, robust backend systems.

Driven by a belief in giving people more control over their privacy, I chose to challenge myself by building an **end-to-end encrypted (E2EE)** chat application, as I believed this to be an area that offers quick early progress but also exposes hard, real-world problems as the system grows.

**Note:** The frontend was implemented with AI assistance to provide a functional interface, allowing the primary engineering focus to remain on the Rust backend and the cryptographic architecture.

# 🔐 Rust & React End-to-End Encrypted Chat

> **A high-security, zero-knowledge messaging platform built effectively from scratch.**

This project demonstrates a production-grade implementation of **End-to-End Encryption** using the Signal Protocol. It features a high-performance **Rust** backend that acts as a blind relay, and a **React/TypeScript** frontend that handles all cryptographic operations locally.

**Value Proposition:** The server *never* sees plaintext messages. Even if the database is compromised, user conversations remain mathematically secure.

---

## 🏗 System Architecture

The system follows a **Zero-Knowledge Architecture**:

1.  **Trust-On-First-Use (TOFU)**: Clients generate Identity Keys and PreKeys locally.
2.  **Key Exchange**: The server acts as a directory service, distributing public keys (Bundles) to facilitate X3DH (Extended Triple Diffie-Hellman) key agreement.
3.  **Double Ratchet Algorithm**: Every message is encrypted with a unique key.
    *   **Forward Secrecy**: Compromising a key reveals nothing about past messages.
    *   **Break-in Recovery**: Compromising a key allows the system to "heal" and secure future messages.
4.  **Local Persistence**: To solve browser limitations, decrypted message history is stored in **IndexedDB** using AES-GCM (Web Crypto API) with a locally managed key.

---

## 🛠 Tech Stack

### Core Security
*   **Protocol**: Signal Protocol (Double Ratchet, X3DH, Sesame).
*   **Library**: `@privacyresearch/libsignal-protocol-typescript`.
*   **Local Encryption**: AES-GCM-256 (Web Crypto API) for protecting local storage.

### Backend (`/server`)
*   **Language**: Rust (2021 Edition).
*   **Framework**: `Axum` (Tokio ecosystem) for high-concurrency WebSocket & REST handling.
*   **Database**: PostgreSQL via `SQLx` (Compile-time verified SQL).
*   **Authentication**: JWT (Access + Refresh tokens).
*   **Infrastructure**: Docker & Docker Compose.

### Frontend (`/web_client`)
*   **Framework**: React 18 + TypeScript.
*   **Build System**: Vite.
*   **State / Storage**: Custom `IndexedDB` wrapper for binary blob storage.
*   **UI**: Tailwind CSS for responsive design.

---

## ✨ Key Features

*   ✅ **Strict E2EE**: No fallback to plaintext. If encryption fails, the message is not sent.
*   ✅ **Self-Encryption**: Your sent messages are encrypted with your own key so you can read them on other devices (Fan-out).
*   ✅ **File Sharing**: Encrypted file uploads (metadata protected).
*   ✅ **Key Replenishment**: Automated One-Time PreKey rotation to prevent key exhaustion.
*   ✅ **Offline Messaging**: Message queuing logic (Simulated via storage).
*   ✅ **Group Logic**: Multi-recipient encryption handling.

---

## 🚀 Getting Started

The easiest way to run the full stack is using Docker Compose.

### Prerequisites
*   Docker & Docker Compose

### Setup

1.  **Clone the repository**
    ```bash
    git clone https://github.com/AntonsRomanceDawn/rust_chat_app.git
    cd rust_chat_app
    ```

2.  **Configure Environment**
    Copy the example configuration:
    ```bash
    cp docker-compose.example.yml docker-compose.yml
    ```
    *Optional: Edit `docker-compose.yml` to set your own passwords/secrets.*

3.  **Run with Docker**
    ```bash
    docker-compose up --build -d
    ```

4.  **Access the App**
    *   Frontend: `http://localhost:5173`
    *   Backend API: `http://localhost:3000`

---

## 🧪 Development Setup (Manual)

If you want to run services individually for development:

### Backend
1.  Navigate to `server/`.
2.  Create `.env`.
2.  Ensure PostgreSQL is running.
3.  Run migrations and server:
    ```bash
    cargo sqlx migrate run
    cargo run
    ```

### Frontend
1.  Navigate to `web_client/`.
2.  Install dependencies:
    ```bash
    npm install
    ```
3.  Start dev server:
    ```bash
    npm run dev
    ```

---

## 🛡 Security considerations

*   **Data Persistence**: If you clear your browser storage (IndexedDB), you will lose your unique private keys. **Previously received messages will become permanently unreadable**, as the keys required to decrypt them are destroyed.
*   **Metadata**: The server knows *who* is talking to *whom* and *when*, but not *what* they are saying.
*   **Browser Security**: Using `IndexedDB` poses risks if the specific machine is compromised (XSS/Malware). In a real-world scenario, this would be paired with a rigorous Content Security Policy (CSP).
*   **Key Trust**: Currently implements strict Trust On First Use (TOFU). Key verification (QR codes / Safety numbers) would be the next roadmap item.

---

## ⚖️ License

MIT
