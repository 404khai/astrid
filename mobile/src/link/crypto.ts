import { gcm } from "@noble/ciphers/aes.js";
import { hkdf } from "@noble/hashes/hkdf.js";
import { sha256 } from "@noble/hashes/sha2.js";

/** The phone link's sealing, as the Mac's `LinkCrypto` (ADR 0039): a key per
 * connection from HKDF-SHA256 over the long-term secret and both hello nonces,
 * then AES-256-GCM with a nonce of the sender's direction and a counter. The
 * receiver expects the next count exactly; ciphertext and tag go on the wire. */
export const PHONE = 0;
export const MAC = 1;

const text = new TextEncoder();

export function sessionKey(secret: Uint8Array, phoneNonce: Uint8Array, macNonce: Uint8Array, label: string): Uint8Array {
  const salt = new Uint8Array(phoneNonce.length + macNonce.length);
  salt.set(phoneNonce);
  salt.set(macNonce, phoneNonce.length);
  return hkdf(sha256, secret, salt, text.encode(`astrid-link/1 ${label}`), 32);
}

export function nonce(direction: number, counter: number): Uint8Array {
  const out = new Uint8Array(12);
  const view = new DataView(out.buffer);
  view.setUint32(0, direction);
  view.setUint32(4, Math.floor(counter / 2 ** 32));
  view.setUint32(8, counter >>> 0);
  return out;
}

export class LinkCipher {
  private sent = 0;
  private received = 0;
  private readonly key: Uint8Array;
  private readonly sends: number;

  constructor(key: Uint8Array, sends: number = PHONE) {
    this.key = key;
    this.sends = sends;
  }

  seal(plaintext: Uint8Array): Uint8Array {
    const out = gcm(this.key, nonce(this.sends, this.sent)).encrypt(plaintext);
    this.sent += 1;
    return out;
  }

  /** Throws on a frame that is forged, replayed or out of order. */
  open(frame: Uint8Array): Uint8Array {
    if (frame.length < 16) throw new Error("short frame");
    const out = gcm(this.key, nonce(this.sends === PHONE ? MAC : PHONE, this.received)).decrypt(frame);
    this.received += 1;
    return out;
  }
}

export const hex = (bytes: Uint8Array) => Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");

export function toBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

export function fromBase64(value: string): Uint8Array {
  const normal = value.replace(/-/g, "+").replace(/_/g, "/");
  const binary = atob(normal + "=".repeat((4 - (normal.length % 4)) % 4));
  return Uint8Array.from(binary, (c) => c.charCodeAt(0));
}
