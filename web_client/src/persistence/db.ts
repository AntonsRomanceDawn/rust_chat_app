import { MessageInfo } from '../types';

const DB_NAME = 'chat_db';
const DB_VERSION = 2; // Bump version to support new stores
const MSG_STORE = 'messages';
const SIGNAL_STORE = 'signal_keys';

export class MessagePersistence {
    private db: IDBDatabase | null = null;

    // ... (init method needs upgrade logic)

    async init(): Promise<void> {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open(DB_NAME, DB_VERSION);

            request.onerror = (event) => {
                console.error("IndexedDB error:", event);
                reject("Failed to open database");
            };

            request.onsuccess = (event) => {
                this.db = (event.target as IDBOpenDBRequest).result;
                resolve();
            };

            request.onupgradeneeded = (event) => {
                const db = (event.target as IDBOpenDBRequest).result;

                // Messages Store
                if (!db.objectStoreNames.contains(MSG_STORE)) {
                    const store = db.createObjectStore(MSG_STORE, { keyPath: 'message_id' });
                    store.createIndex('room_id', 'room_id', { unique: false });
                    store.createIndex('created_at', 'created_at', { unique: false });
                }

                // Signal Protocol Store (Key/Value store for sessions, prekeys, etc.)
                if (!db.objectStoreNames.contains(SIGNAL_STORE)) {
                    db.createObjectStore(SIGNAL_STORE, { keyPath: 'key' });
                }
            };
        });
    }

    // --- Signal Helpers ---

    async putSignalKey(key: string, value: any): Promise<void> {
        if (!this.db) await this.init();
        return new Promise((resolve, reject) => {
            const tx = this.db!.transaction([SIGNAL_STORE], 'readwrite');
            const store = tx.objectStore(SIGNAL_STORE);
            const req = store.put({ key, value });
            req.onsuccess = () => resolve();
            req.onerror = () => reject(req.error);
        });
    }

    async getSignalKey(key: string): Promise<any | undefined> {
        if (!this.db) await this.init();
        return new Promise((resolve, reject) => {
            const tx = this.db!.transaction([SIGNAL_STORE], 'readonly');
            const store = tx.objectStore(SIGNAL_STORE);
            const req = store.get(key);
            req.onsuccess = () => resolve(req.result ? req.result.value : undefined);
            req.onerror = () => reject(req.error);
        });
    }

    async removeSignalKey(key: string): Promise<void> {
        if (!this.db) await this.init();
        return new Promise((resolve, reject) => {
            const tx = this.db!.transaction([SIGNAL_STORE], 'readwrite');
            const store = tx.objectStore(SIGNAL_STORE);
            const req = store.delete(key);
            req.onsuccess = () => resolve();
            req.onerror = () => reject(req.error);
        });
    }

    // Helper to clear sessions by prefix (SignalStore requirement)
    async removeSignalKeysByPrefix(prefix: string): Promise<void> {
        if (!this.db) await this.init();
        return new Promise((resolve, reject) => {
            const tx = this.db!.transaction([SIGNAL_STORE], 'readwrite');
            const store = tx.objectStore(SIGNAL_STORE);
            // Iterate all keys (not efficient for huge DBs, but fine for Signal keys)
            // A cursor or specific range would be better if we structure keys smartly.
            // Given standard signal key usage, cursor is acceptable.
            const req = store.openCursor();
            req.onsuccess = (e) => {
                const cursor = (e.target as IDBRequest).result as IDBCursorWithValue;
                if (cursor) {
                    if (String(cursor.key).startsWith(prefix)) {
                        cursor.delete();
                    }
                    cursor.continue();
                } else {
                    resolve();
                }
            };
            req.onerror = () => reject(req.error);
        });
    }

    async saveMessage(message: MessageInfo & { room_id: string }): Promise<void> {
        if (!this.db) await this.init();
        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([MSG_STORE], 'readwrite');
            const store = transaction.objectStore(MSG_STORE);
            const request = store.put(message);

            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    async getMessage(messageId: string): Promise<(MessageInfo & { room_id: string }) | undefined> {
        if (!this.db) await this.init();
        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([MSG_STORE], 'readonly');
            const store = transaction.objectStore(MSG_STORE);
            const request = store.get(messageId);

            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
    }

    async getMessagesForRoom(roomId: string): Promise<(MessageInfo & { room_id: string })[]> {
        if (!this.db) await this.init();
        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([MSG_STORE], 'readonly');
            const store = transaction.objectStore(MSG_STORE);
            const index = store.index('room_id');
            const request = index.getAll(roomId); // Note: returns all, sorting might be needed in memory if not using cursor

            request.onsuccess = () => resolve(request.result || []);
            request.onerror = () => reject(request.error);
        });
    }
}

export const messageDb = new MessagePersistence();
