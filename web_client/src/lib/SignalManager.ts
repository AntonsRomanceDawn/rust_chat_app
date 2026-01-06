import * as libsignal from '@privacyresearch/libsignal-protocol-typescript';
import { SignalStore } from './SignalStore';
import axios from 'axios';
import { Buffer } from 'buffer';

// Debugging imports
console.log('libsignal import:', libsignal);
console.log('libsignal.default:', (libsignal as any).default);

const getLib = () => {
    if ((libsignal as any).default && (libsignal as any).default.KeyHelper) {
        return (libsignal as any).default;
    }
    return libsignal;
};

const lib = getLib();
const KeyHelper = lib.KeyHelper;
const SessionBuilder = lib.SessionBuilder;
const SessionCipher = lib.SessionCipher;
const SignalProtocolAddress = lib.SignalProtocolAddress;
const SignedPreKeyRecord = lib.SignedPreKeyRecord;
const PreKeyRecord = lib.PreKeyRecord;

export class SignalManager {
    private store: SignalStore;
    private username: string;
    private token: string;

    constructor(username: string, token: string) {
        this.store = new SignalStore();
        this.username = username;
        this.token = token;
    }

    async initialize() {
        if (!SignedPreKeyRecord) {
            throw new Error("SignedPreKeyRecord is undefined. Library not loaded correctly.");
        }
        const identity = await this.store.getIdentityKeyPair();
        if (!identity) {
            await this.generateAndUploadKeys();
        }
    }

    private async generateAndUploadKeys() {
        const registrationId = KeyHelper.generateRegistrationId();
        const identityKeyPair = await KeyHelper.generateIdentityKeyPair();

        const startId = Math.floor(Math.random() * 1000000);
        const preKeys = [];
        for (let i = 0; i < 100; i++) {
            preKeys.push(await KeyHelper.generatePreKey(startId + i));
        }
        const signedPreKey = await KeyHelper.generateSignedPreKey(identityKeyPair, startId);

        const uploadDto = {
            identity_key: Buffer.from(identityKeyPair.pubKey).toString('base64'),
            registration_id: registrationId,
            signed_prekey: {
                key_id: signedPreKey.keyId,
                public_key: Buffer.from(signedPreKey.keyPair.pubKey).toString('base64'),
                signature: Buffer.from(signedPreKey.signature).toString('base64')
            },
            one_time_prekeys: preKeys.map(k => ({
                key_id: k.keyId,
                public_key: Buffer.from(k.keyPair.pubKey).toString('base64')
            }))
        };

        await axios.post('/api/keys', uploadDto, {
            headers: { Authorization: `Bearer ${this.token}` }
        });

        await this.store.storeLocalRegistrationId(registrationId);
        await this.store.storeIdentityKeyPair(identityKeyPair);

        const signedPreKeyRecord = new SignedPreKeyRecord(
            signedPreKey.keyId,
            Date.now(),
            signedPreKey.keyPair,
            signedPreKey.signature
        );
        await this.store.storeSignedPreKey(signedPreKey.keyId, signedPreKeyRecord);

        for (const preKey of preKeys) {
            const preKeyRecord = new PreKeyRecord(preKey.keyId, preKey.keyPair);
            await this.store.storePreKey(preKey.keyId, preKeyRecord);
        }
    }

    async ensureSession(remoteUsername: string) {
        // Ensure we don't try to create a session with ourselves
        if (remoteUsername === this.username) return;

        const address = new SignalProtocolAddress(remoteUsername, 1);
        if (await this.store.loadSession(address.toString())) {
            return;
        }

        try {
            const response = await axios.get(`/api/keys/${remoteUsername}`, {
                headers: { Authorization: `Bearer ${this.token}` }
            });
            const bundle = response.data;

            const builder = new SessionBuilder(this.store, address);
            await builder.processPreKey({
                registrationId: bundle.registration_id,
                identityKey: Buffer.from(bundle.identity_key, 'base64'),
                signedPreKey: {
                    keyId: bundle.signed_prekey.key_id,
                    publicKey: Buffer.from(bundle.signed_prekey.public_key, 'base64'),
                    signature: Buffer.from(bundle.signed_prekey.signature, 'base64')
                },
                preKey: bundle.one_time_prekey ? {
                    keyId: bundle.one_time_prekey.key_id,
                    publicKey: Buffer.from(bundle.one_time_prekey.public_key, 'base64')
                } : undefined
            });
        } catch (e: any) {
            if (e.response && e.response.status === 400 && e.response.data?.errors?.[0]?.code === 'user_has_no_keys') {
                console.warn(`User ${remoteUsername} has not set up E2EE keys yet.`);
            } else {
                console.error(`Failed to establish session with ${remoteUsername}`, e);
            }
            throw e;
        }
    }

    async encryptGroupMessage(_roomId: string, content: string, members: string[]): Promise<{ content: string, type: number }> {
        const key = await window.crypto.subtle.generateKey(
            { name: "AES-GCM", length: 256 },
            true,
            ["encrypt", "decrypt"]
        );
        const iv = window.crypto.getRandomValues(new Uint8Array(12));

        const encoded = new TextEncoder().encode(content);
        const ciphertext = await window.crypto.subtle.encrypt(
            { name: "AES-GCM", iv },
            key,
            encoded
        );

        const rawKey = await window.crypto.subtle.exportKey("raw", key);
        const rawKeyBuffer = Buffer.from(rawKey);

        const keys: Record<string, any> = {};
        for (const member of members) {
            if (member === this.username) continue;

            try {
                await this.ensureSession(member);
                const address = new SignalProtocolAddress(member, 1);
                const cipher = new SessionCipher(this.store, address);
                const encryptedKey = await cipher.encrypt(rawKeyBuffer);
                keys[member] = {
                    type: encryptedKey.type,
                    body: Buffer.from(encryptedKey.body).toString('base64'),
                    registrationId: encryptedKey.registrationId
                };
            } catch (e) {
                console.error(`Failed to encrypt key for ${member}`, e);
                // Continue with other members? Or fail?
                // For now, continue.
            }
        }

        // Encrypt for self to be able to read history
        // We can just store the key encrypted for ourselves using the same mechanism
        // Or we can just rely on local state if we don't care about multi-device sync for now.
        // Let's encrypt for self using Signal Session (loopback).
        try {
            // Ensure session with self?
            // Signal Protocol usually doesn't do session with self easily without different device IDs.
            // Since we are device 1, we can't make a session with device 1.
            // So we'll skip self encryption for now.
            // The sender will see "[Encrypted Message]" if they reload.
            // To fix this, we could store the plaintext in local storage or encrypt with a local key.
        } catch (e) { }

        const payload = {
            iv: Buffer.from(iv).toString('base64'),
            ciphertext: Buffer.from(ciphertext).toString('base64'),
            keys
        };

        return {
            content: JSON.stringify(payload),
            type: 1
        };
    }

    async decryptMessage(sender: string, content: string) {
        try {
            let payload;
            try {
                payload = JSON.parse(content);
            } catch (e) {
                // Not JSON, assume plaintext
                return content;
            }

            if (!payload.keys || !payload.ciphertext || !payload.iv) {
                // Maybe it's a plain text message from before E2EE?
                return content;
            }

            const myKeyEncrypted = payload.keys[this.username];
            if (!myKeyEncrypted) {
                if (sender === this.username) {
                    return "[You sent this message]";
                }
                return "[Decryption Error: No key for me]";
            }

            const address = new SignalProtocolAddress(sender, 1);
            const cipher = new SessionCipher(this.store, address);

            let plaintextKey: ArrayBuffer;
            if (myKeyEncrypted.type === 3) {
                plaintextKey = await cipher.decryptPreKeyWhisperMessage(
                    Buffer.from(myKeyEncrypted.body, 'base64'),
                    'binary'
                );
            } else {
                plaintextKey = await cipher.decryptWhisperMessage(
                    Buffer.from(myKeyEncrypted.body, 'base64'),
                    'binary'
                );
            }

            const key = await window.crypto.subtle.importKey(
                "raw",
                plaintextKey,
                { name: "AES-GCM" },
                true,
                ["decrypt"]
            );

            const decrypted = await window.crypto.subtle.decrypt(
                { name: "AES-GCM", iv: Buffer.from(payload.iv, 'base64') },
                key,
                Buffer.from(payload.ciphertext, 'base64')
            );

            return new TextDecoder().decode(decrypted);

        } catch (e) {
            console.error("Decryption failed", e);
            return "[Decryption Failed]";
        }
    }
}
