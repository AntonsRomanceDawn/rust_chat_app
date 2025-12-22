import {
    SessionRecord,
    SignalProtocolAddress,
    PreKeyRecord,
    SignedPreKeyRecord,
    IdentityKeyPair,
    SessionStore,
    IdentityKeyStore,
    PreKeyStore,
    SignedPreKeyStore,
    SenderKeyStore,
    SenderKeyRecord,
    Direction
} from '@privacyresearch/libsignal-protocol-typescript';

export class SignalStore implements SessionStore, IdentityKeyStore, PreKeyStore, SignedPreKeyStore, SenderKeyStore {
    private _store: Record<string, any> = {};

    constructor() {
        this.load();
    }

    private load() {
        const stored = localStorage.getItem('signal_store_v2');
        if (stored) {
            this._store = JSON.parse(stored);
        }
    }

    private save() {
        localStorage.setItem('signal_store_v2', JSON.stringify(this._store));
    }

    private get(key: string): any {
        return this._store[key];
    }

    private put(key: string, value: any) {
        this._store[key] = value;
        this.save();
    }

    private remove(key: string) {
        delete this._store[key];
        this.save();
    }

    private toArrayBuffer(thing: string | Buffer): ArrayBuffer {
        if (thing === undefined || thing === null) return undefined;
        if (typeof thing === 'string') {
            return new Uint8Array(Buffer.from(thing, 'base64')).buffer;
        }
        return new Uint8Array(thing).buffer;
    }

    // IdentityKeyStore
    async getIdentityKeyPair(): Promise<IdentityKeyPair | undefined> {
        const kp = this.get('identityKey');
        if (!kp) return undefined;
        return {
            pubKey: this.toArrayBuffer(kp.pubKey),
            privKey: this.toArrayBuffer(kp.privKey)
        };
    }

    async getLocalRegistrationId(): Promise<number | undefined> {
        return this.get('registrationId');
    }

    private ensureAddressString(identifier: string): string {
        if (!identifier.includes('.')) {
            return `${identifier}.1`;
        }
        return identifier;
    }

    async saveIdentity(identifier: string, identityKey: ArrayBuffer): Promise<boolean> {
        const address = SignalProtocolAddress.fromString(this.ensureAddressString(identifier));
        const existing = this.get('identity_' + address.getName());
        this.put('identity_' + address.getName(), Buffer.from(identityKey).toString('base64'));
        return !!existing && existing !== Buffer.from(identityKey).toString('base64');
    }

    async isTrustedIdentity(identifier: string, identityKey: ArrayBuffer, direction: Direction): Promise<boolean> {
        const address = SignalProtocolAddress.fromString(this.ensureAddressString(identifier));
        const existing = this.get('identity_' + address.getName());
        if (!existing) {
            return true; // Trust on first use
        }
        return existing === Buffer.from(identityKey).toString('base64');
    }

    async loadIdentityKey(identifier: string): Promise<ArrayBuffer | undefined> {
        const address = SignalProtocolAddress.fromString(this.ensureAddressString(identifier));
        const key = this.get('identity_' + address.getName());
        if (!key) return undefined;
        return this.toArrayBuffer(key);
    }

    // PreKeyStore
    async loadPreKey(keyId: number): Promise<PreKeyRecord | undefined> {
        const key = this.get('preKey_' + keyId);
        if (!key) return undefined;
        return PreKeyRecord.deserialize(Buffer.from(key, 'base64').toString('binary'));
    }

    async storePreKey(keyId: number, keyRecord: PreKeyRecord): Promise<void> {
        this.put('preKey_' + keyId, Buffer.from(keyRecord.serialize()).toString('base64'));
    }

    async removePreKey(keyId: number): Promise<void> {
        this.remove('preKey_' + keyId);
    }

    // SignedPreKeyStore
    async loadSignedPreKey(keyId: number): Promise<SignedPreKeyRecord | undefined> {
        const key = this.get('signedPreKey_' + keyId);
        if (!key) return undefined;
        return SignedPreKeyRecord.deserialize(Buffer.from(key, 'base64').toString('binary'));
    }

    async storeSignedPreKey(keyId: number, keyRecord: SignedPreKeyRecord): Promise<void> {
        this.put('signedPreKey_' + keyId, Buffer.from(keyRecord.serialize()).toString('base64'));
    }

    async removeSignedPreKey(keyId: number): Promise<void> {
        this.remove('signedPreKey_' + keyId);
    }

    // SessionStore
    async loadSession(identifier: string): Promise<SessionRecord | undefined> {
        const address = SignalProtocolAddress.fromString(this.ensureAddressString(identifier));
        const key = this.get('session_' + address.getName() + '.' + address.getDeviceId());
        if (!key) return undefined;
        return SessionRecord.deserialize(Buffer.from(key, 'base64').toString('binary'));
    }

    async storeSession(identifier: string, record: SessionRecord): Promise<void> {
        const address = SignalProtocolAddress.fromString(this.ensureAddressString(identifier));
        this.put('session_' + address.getName() + '.' + address.getDeviceId(), Buffer.from(record.serialize()).toString('base64'));
    }

    async getSubDeviceSessions(identifier: string): Promise<number[]> {
        // Assuming only device ID 1 for now
        const address = SignalProtocolAddress.fromString(identifier);
        if (this.get('session_' + address.getName() + '.1')) {
            return [1];
        }
        return [];
    }

    // SenderKeyStore
    async saveSenderKey(senderKeyName: SignalProtocolAddress, senderKeyId: string, senderKeyRecord: SenderKeyRecord): Promise<void> {
        const key = `senderKey_${senderKeyName.getName()}_${senderKeyName.getDeviceId()}_${senderKeyId}`;
        this.put(key, Buffer.from(senderKeyRecord.serialize()).toString('base64'));
    }

    async loadSenderKey(senderKeyName: SignalProtocolAddress, senderKeyId: string): Promise<SenderKeyRecord | undefined> {
        const key = `senderKey_${senderKeyName.getName()}_${senderKeyName.getDeviceId()}_${senderKeyId}`;
        const data = this.get(key);
        if (!data) return undefined;
        return SenderKeyRecord.deserialize(Buffer.from(data, 'base64').toString('binary'));
    }

    // Helpers for initialization
    async storeIdentityKeyPair(identityKeyPair: IdentityKeyPair): Promise<void> {
        this.put('identityKey', {
            pubKey: Buffer.from(identityKeyPair.pubKey).toString('base64'),
            privKey: Buffer.from(identityKeyPair.privKey).toString('base64')
        });
    }

    async storeLocalRegistrationId(registrationId: number): Promise<void> {
        this.put('registrationId', registrationId);
    }
}
