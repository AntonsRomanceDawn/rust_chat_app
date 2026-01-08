import {
    StorageType,
    Direction,
    KeyPairType
} from '@privacyresearch/libsignal-protocol-typescript';
import { messageDb } from '../persistence/db';

function arrayBufferToBase64(buffer: ArrayBuffer): string {
    let binary = '';
    const bytes = new Uint8Array(buffer);
    const len = bytes.byteLength;
    for (let i = 0; i < len; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return window.btoa(binary);
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binary_string = window.atob(base64);
    const len = binary_string.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
        bytes[i] = binary_string.charCodeAt(i);
    }
    return bytes.buffer;
}

interface SerializedKeyPair {
    pubKey: string;
    privKey: string;
}

export class SignalProtocolStore implements StorageType {
    constructor() { }

    // Helper to migrate from localStorage if needed
    private async getOrMigrate(key: string): Promise<any | undefined> {
        let val = await messageDb.getSignalKey(key);
        if (val === undefined) {
            const lsVal = localStorage.getItem(key);
            if (lsVal !== null) {
                // Found in localStorage, migrate to IDB
                try {
                    // Sessions should remain strings (libsignal expects serialized string).
                    // Other keys were wrapped in our own JSON structure, so we parse them.
                    if (key.startsWith('session_')) {
                        val = lsVal;
                    } else {
                        val = JSON.parse(lsVal);
                    }
                } catch {
                    val = lsVal;
                }
                await messageDb.putSignalKey(key, val);
                localStorage.removeItem(key); // Cleanup
            }
        }
        return val;
    }

    async getIdentityKeyPair(): Promise<KeyPairType | undefined> {
        const kp = await this.getOrMigrate('identityKey');
        if (kp) {
            return {
                pubKey: base64ToArrayBuffer(kp.pubKey),
                privKey: base64ToArrayBuffer(kp.privKey)
            };
        }
        return undefined;
    }

    async getLocalRegistrationId(): Promise<number | undefined> {
        const rid = await this.getOrMigrate('registrationId');
        if (rid !== undefined) {
            // In localStorage it was string "123". In IDB it can be number 123.
            return typeof rid === 'string' ? parseInt(rid, 10) : rid;
        }
        return undefined;
    }

    async putIdentityKeyPair(member: KeyPairType): Promise<void> {
        const serialized: SerializedKeyPair = {
            pubKey: arrayBufferToBase64(member.pubKey),
            privKey: arrayBufferToBase64(member.privKey)
        };
        await messageDb.putSignalKey('identityKey', serialized);
    }

    async putLocalRegistrationId(registrationId: number): Promise<void> {
        await messageDb.putSignalKey('registrationId', registrationId);
    }

    async isTrustedIdentity(identifier: string, identityKey: ArrayBuffer, direction: Direction): Promise<boolean> {
        return !!(identifier && identityKey && direction);
    }

    async saveIdentity(identifier: string, identityKey: ArrayBuffer): Promise<boolean> {
        return !!(identifier && identityKey);
    }

    async getIdentity(_identifier: string): Promise<ArrayBuffer | undefined> {
        return undefined;
    }

    async loadPreKey(keyId: string | number): Promise<KeyPairType | undefined> {
        const key = await this.getOrMigrate(`preKey_${keyId}`);
        if (key) {
            return {
                pubKey: base64ToArrayBuffer(key.pubKey),
                privKey: base64ToArrayBuffer(key.privKey)
            };
        }
        return undefined;
    }

    async storePreKey(keyId: string | number, keyPair: KeyPairType): Promise<void> {
        const serialized: SerializedKeyPair = {
            pubKey: arrayBufferToBase64(keyPair.pubKey),
            privKey: arrayBufferToBase64(keyPair.privKey)
        };
        await messageDb.putSignalKey(`preKey_${keyId}`, serialized);
    }

    async removePreKey(keyId: string | number): Promise<void> {
        await messageDb.removeSignalKey(`preKey_${keyId}`);
        localStorage.removeItem(`preKey_${keyId}`);
    }

    async loadSignedPreKey(keyId: string | number): Promise<KeyPairType | undefined> {
        const key = await this.getOrMigrate(`signedPreKey_${keyId}`);
        if (key) {
            return {
                pubKey: base64ToArrayBuffer(key.pubKey),
                privKey: base64ToArrayBuffer(key.privKey)
            };
        }
        return undefined;
    }

    async storeSignedPreKey(keyId: string | number, keyPair: KeyPairType): Promise<void> {
        const serialized: SerializedKeyPair = {
            pubKey: arrayBufferToBase64(keyPair.pubKey),
            privKey: arrayBufferToBase64(keyPair.privKey)
        };
        await messageDb.putSignalKey(`signedPreKey_${keyId}`, serialized);
    }

    async removeSignedPreKey(keyId: string | number): Promise<void> {
        await messageDb.removeSignalKey(`signedPreKey_${keyId}`);
        localStorage.removeItem(`signedPreKey_${keyId}`);
    }

    async loadSession(identifier: string): Promise<string | undefined> {
        let key = await this.getOrMigrate(`session_${identifier}`);
        if (key) {
            // Recovery check: If a session was incorrectly migrated as a parsed Object,
            // convert it back to a JSON string because libsignal expects the serialized string.
            if (typeof key === 'object') {
                return JSON.stringify(key);
            }
            return key;
        }
        return undefined;
    }

    async storeSession(identifier: string, record: string): Promise<void> {
        await messageDb.putSignalKey(`session_${identifier}`, record);
    }

    async removeSession(identifier: string): Promise<void> {
        await messageDb.removeSignalKey(`session_${identifier}`);
        localStorage.removeItem(`session_${identifier}`);
    }

    async removeAllSessions(identifier: string): Promise<void> {
        await messageDb.removeSignalKeysByPrefix(`session_${identifier}`);

        // Also cleanup localStorage
        const keys = [];
        for (let i = 0; i < localStorage.length; i++) {
            const key = localStorage.key(i);
            if (key && key.startsWith(`session_${identifier}`)) {
                keys.push(key);
            }
        }
        keys.forEach(k => localStorage.removeItem(k));
    }
}
