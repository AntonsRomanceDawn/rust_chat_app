import {
    KeyHelper,
    SessionBuilder,
    SessionCipher,
    SignalProtocolAddress,
    PreKeyPairType
} from '@privacyresearch/libsignal-protocol-typescript';
import { SignalProtocolStore } from './SignalStore';
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

export class SignalManager {
    store: SignalProtocolStore;
    username: string;
    apiClient: any;

    constructor(username: string, apiClient: any) {
        this.store = new SignalProtocolStore();
        this.username = username;
        this.apiClient = apiClient;
    }

    async init() {
        const identity = await this.store.getIdentityKeyPair();
        if (identity) {
            console.log('Signal keys exist locally, checking for replenishment...');
            await this.checkAndReplenishKeys();
            return;
        }

        console.log('Generating new Signal keys...');
        const registrationId = KeyHelper.generateRegistrationId();
        const identityKeyPair = await KeyHelper.generateIdentityKeyPair();

        const preKeys: PreKeyPairType[] = [];
        const preKeyStartId = 1;
        const BATCH_SIZE = 100;

        for (let i = 0; i < BATCH_SIZE; i++) {
            const key = await KeyHelper.generatePreKey(preKeyStartId + i);
            preKeys.push(key);
            await this.store.storePreKey(key.keyId, key.keyPair);
        }

        // Save the next ID to use for replenishment
        await messageDb.putSignalKey('nextPreKeyId', (preKeyStartId + BATCH_SIZE));

        const signedPreKeyId = 1;
        const signedPreKey = await KeyHelper.generateSignedPreKey(
            identityKeyPair,
            signedPreKeyId
        );
        await this.store.storeSignedPreKey(signedPreKeyId, signedPreKey.keyPair);

        await this.store.putIdentityKeyPair(identityKeyPair);
        await this.store.putLocalRegistrationId(registrationId);

        const payload = {
            identity_key: arrayBufferToBase64(identityKeyPair.pubKey),
            registration_id: registrationId,
            signed_prekey: {
                key_id: signedPreKeyId,
                public_key: arrayBufferToBase64(signedPreKey.keyPair.pubKey),
                signature: arrayBufferToBase64(signedPreKey.signature)
            },
            one_time_prekeys: preKeys.map(k => ({
                key_id: k.keyId,
                public_key: arrayBufferToBase64(k.keyPair.pubKey)
            }))
        };

        try {
            await this.apiClient.post('/keys', payload);
            console.log("Keys uploaded successfully.");
        } catch (e) {
            console.error("Failed to upload keys", e);
        }
    }

    async checkAndReplenishKeys() {
        try {
            // 1. Check count from server
            const res = await this.apiClient.get('/keys/status/count');
            const count = res.data.count;
            const LOW_THRESHOLD = 20;
            const REPLENISH_AMOUNT = 50;

            if (count < LOW_THRESHOLD) {
                console.log(`Key count low (${count}), replenishing ${REPLENISH_AMOUNT} keys...`);

                // 2. Determine start ID
                // Migration logic for nextPreKeyId
                let nextIdVal = await messageDb.getSignalKey('nextPreKeyId');
                if (nextIdVal === undefined) {
                    const lsVal = localStorage.getItem('nextPreKeyId');
                    if (lsVal) {
                        nextIdVal = parseInt(lsVal);
                        await messageDb.putSignalKey('nextPreKeyId', nextIdVal);
                        localStorage.removeItem('nextPreKeyId');
                    }
                }

                let nextId = typeof nextIdVal === 'number' ? nextIdVal : 1;
                /* istanbul ignore next */
                if (isNaN(nextId)) nextId = 1;

                // 3. Generate new keys
                const newKeys: PreKeyPairType[] = [];
                for (let i = 0; i < REPLENISH_AMOUNT; i++) {
                    const keyId = nextId + i;
                    const key = await KeyHelper.generatePreKey(keyId);
                    newKeys.push(key);
                    await this.store.storePreKey(keyId, key.keyPair);
                }

                // 4. Update local state
                await messageDb.putSignalKey('nextPreKeyId', (nextId + REPLENISH_AMOUNT));

                // 5. Construct partial upload payload
                // Note: The backend expects the full structure, so we must re-send identity/signed keys
                // OR adapt the backend. Assuming we just resend the existing ones for compliance.
                const identityKeyPair = await this.store.getIdentityKeyPair();
                const registrationId = await this.store.getLocalRegistrationId();
                const signedPreKeyId = 1; // Assuming we keep the same signed prekey for now (rotation is separate topic)
                const signedPreKeyPair = await this.store.loadSignedPreKey(signedPreKeyId);

                if (!identityKeyPair || !registrationId || !signedPreKeyPair) {
                    console.error("Missing local identity/signed keys, cannot replenish.");
                    return;
                }

                // We need the signature... we didn't store it in storeSignedPreKey above,
                // strictly speaking we should have stored the FULL object or just re-sign it.
                // For simplicity, let's re-sign the SAME key to get a valid signature
                // (or generate a new signed prekey, which is actually better for security rotation).

                // ROTATION BONUS: Let's rotate the signed prekey too while we are at it.
                const newSignedPreKeyId = Math.floor(Date.now() / 1000) % 100000; // Simple random ID
                const newSignedKey = await KeyHelper.generateSignedPreKey(identityKeyPair, newSignedPreKeyId);
                await this.store.storeSignedPreKey(newSignedPreKeyId, newSignedKey.keyPair);

                const payload = {
                    identity_key: arrayBufferToBase64(identityKeyPair.pubKey),
                    registration_id: registrationId,
                    signed_prekey: {
                        key_id: newSignedPreKeyId,
                        public_key: arrayBufferToBase64(newSignedKey.keyPair.pubKey),
                        signature: arrayBufferToBase64(newSignedKey.signature)
                    },
                    one_time_prekeys: newKeys.map(k => ({
                        key_id: k.keyId,
                        public_key: arrayBufferToBase64(k.keyPair.pubKey)
                    }))
                };

                await this.apiClient.post('/keys', payload);
                console.log("Keys replenished and signed prekey rotated.");
            }
        } catch (e) {
            console.error("Failed to check/replenish keys", e);
        }
    }

    async ensureSession(remoteUsername: string) {
        const address = new SignalProtocolAddress(remoteUsername, 1);
        const session = await this.store.loadSession(address.toString());
        // For SignalProtocolStore we implemented session as string.
        // check if it's not undefined/empty
        if (session) return;

        console.log(`No session for ${remoteUsername}, fetching bundle...`);
        try {
            const response = await this.apiClient.get(`/keys/${remoteUsername}`);
            const bundle = response.data;

            const sessionBuilder = new SessionBuilder(this.store, address);
            const preKeyBundle = {
                identityKey: base64ToArrayBuffer(bundle.identity_key),
                registrationId: bundle.registration_id,
                signedPreKey: {
                    keyId: bundle.signed_prekey.key_id,
                    publicKey: base64ToArrayBuffer(bundle.signed_prekey.public_key),
                    signature: base64ToArrayBuffer(bundle.signed_prekey.signature)
                },
                preKey: bundle.one_time_prekey ? {
                    keyId: bundle.one_time_prekey.key_id,
                    publicKey: base64ToArrayBuffer(bundle.one_time_prekey.public_key)
                } : undefined
            };

            await sessionBuilder.processPreKey(preKeyBundle);
            console.log(`Session established with ${remoteUsername}`);
        } catch (e) {
            console.error(`Error establishing session with ${remoteUsername}`, e);
            throw e;
        }
    }

    async encryptGroupMessage(recipients: string[], message: string): Promise<string> {
        const encryptedPayloads: Record<string, any> = {};

        for (const recipient of recipients) {
            if (recipient === this.username) continue; // Skip self for Signal Session

            // Propagate errors immediately to fail the whole send if any recipient cannot be reached
            await this.ensureSession(recipient);
            const address = new SignalProtocolAddress(recipient, 1);
            const cipher = new SessionCipher(this.store, address);

            const encoder = new TextEncoder();
            const plaintext = encoder.encode(message).buffer;

            const ciphertext = await cipher.encrypt(plaintext);

            // Safe handling: Base64 encode the binary string body to avoid JSON issues
            encryptedPayloads[recipient] = {
                type: ciphertext.type,
                body: window.btoa(ciphertext.body || '')
            };
        }

        // Handle Self-Encryption for History/Echo using a simpler method
        // This avoids the Signal Session ratchet loopback issue on single device
        try {
            const selfPayload = await this.encryptForSelf(message);
            if (selfPayload) {
                // Use a special key for self payload in the map
                encryptedPayloads[this.username] = selfPayload;
            }
        } catch (e) {
            console.error("Failed to encrypt for self", e);
        }

        return JSON.stringify(encryptedPayloads);
    }

    async decryptGroupMessage(senderUsername: string, packedContent: string): Promise<string> {
        console.log(`Decrypting message from ${senderUsername}`);
        try {
            let payloads;
            try {
                console.log("Parsing packed content:", packedContent);
                payloads = JSON.parse(packedContent);
            } catch (e) {
                console.error("JSON Parse failed on content:", packedContent, e);
                throw new Error("Message content is not valid JSON (Strict Mode: Plaintext fallback disabled).");
            }

            const myPayload = payloads[this.username];
            if (!myPayload) {
                console.log("No payload for me in the message:", myPayload);
                return "[Unreadable/Not for me]";
            }

            // Special handling for Self Encryption
            if (senderUsername === this.username && myPayload.type === 'self') {
                try {
                    console.log("Performing self-decryption for message.");
                    return await this.decryptForSelf(myPayload.body);
                } catch (err) {
                    console.error("Self-decryption failed:", err);
                    throw err;
                }
            }

            const address = new SignalProtocolAddress(senderUsername, 1);
            const cipher = new SessionCipher(this.store, address);

            // Convert Base64 to ArrayBuffer for consumption by libsignal
            // This ensures compatibility with stricter types and avoids binary string ambiguity
            const ciphertextBuffer = base64ToArrayBuffer(myPayload.body);

            let buffer: ArrayBuffer;
            if (myPayload.type === 3) {
                console.log("Decrypting PreKey Whisper Message");
                buffer = await cipher.decryptPreKeyWhisperMessage(ciphertextBuffer);
            } else {
                console.log("Decrypting Whisper Message");
                buffer = await cipher.decryptWhisperMessage(ciphertextBuffer);
            }


            const decoder = new TextDecoder();
            return decoder.decode(buffer);
        } catch (e) {
            console.error("Decryption failed", e);
            throw new Error(`Decryption failed: ${(e as Error).message}`);
        }
    }

    // --- Self Encryption Helpers (AES-GCM) ---

    // Generates or retrieves a persistent symmetric key for local history
    private async getSelfKey(): Promise<CryptoKey> {
        const storageKey = `self_encryption_key_${this.username}`;

        let rawKey = await messageDb.getSignalKey(storageKey);
        if (!rawKey) {
            const lsKey = localStorage.getItem(storageKey);
            if (lsKey) {
                rawKey = lsKey;
                await messageDb.putSignalKey(storageKey, rawKey);
                localStorage.removeItem(storageKey);
            }
        }

        if (rawKey) {
            const keyData = base64ToArrayBuffer(rawKey);
            return window.crypto.subtle.importKey(
                "raw",
                keyData,
                "AES-GCM",
                true,
                ["encrypt", "decrypt"]
            );
        } else {
            const key = await window.crypto.subtle.generateKey(
                { name: "AES-GCM", length: 256 } as AesKeyGenParams,
                true,
                ["encrypt", "decrypt"]
            );
            const exported = await window.crypto.subtle.exportKey("raw", key as CryptoKey);
            await messageDb.putSignalKey(storageKey, arrayBufferToBase64(exported));
            return key as CryptoKey;
        }
    }

    private async encryptForSelf(message: string): Promise<{ type: string, body: string }> {
        const key = await this.getSelfKey();
        const iv = window.crypto.getRandomValues(new Uint8Array(12));
        const encoder = new TextEncoder();
        const encoded = encoder.encode(message);

        const ciphertext = await window.crypto.subtle.encrypt(
            { name: "AES-GCM", iv: iv },
            key,
            encoded
        );

        // Pack IV and Ciphertext together: IV (12 bytes) + Ciphertext
        const combined = new Uint8Array(iv.length + ciphertext.byteLength);
        combined.set(iv);
        combined.set(new Uint8Array(ciphertext), iv.length);

        return {
            type: 'self',
            body: arrayBufferToBase64(combined.buffer)
        };
    }

    private async decryptForSelf(base64Body: string): Promise<string> {
        const key = await this.getSelfKey();
        const combined = new Uint8Array(base64ToArrayBuffer(base64Body));

        // Extract IV
        const iv = combined.slice(0, 12);
        const data = combined.slice(12);

        const decrypted = await window.crypto.subtle.decrypt(
            { name: "AES-GCM", iv: iv },
            key,
            data
        );

        const decoder = new TextDecoder();
        return decoder.decode(decrypted);
    }
}
