import { LinkCipher, PHONE, fromBase64, sessionKey, toBase64 } from "./crypto.ts";
import type { PairingOffer } from "./pairing.ts";

/** A Mac this phone is paired with (ADR 0039). Kept in secure storage. */
export type Pairing = { device: string; key: string; hosts: string[]; port: number; mac: string };

export type LinkStatus = "connecting" | "online" | "offline";

const encoder = new TextEncoder();
const decoder = new TextDecoder();
/** Seconds to reach one address before trying the next. */
const CONNECT_TIMEOUT = 4000;
/** Seconds for the Mac's hello and first reply. */
const HANDSHAKE_TIMEOUT = 10000;

type Socket = {
  binaryType: string;
  readyState: number;
  onopen: (() => void) | null;
  onmessage: ((event: { data: unknown }) => void) | null;
  onclose: (() => void) | null;
  onerror: (() => void) | null;
  send(data: string | ArrayBuffer | Uint8Array): void;
  close(): void;
};

export type LinkDeps = {
  random(count: number): Uint8Array;
  socket(url: string): Socket;
};

/** One open, sealed connection to the Mac. */
class Connection {
  private cipher: LinkCipher | null = null;
  onMessage: (message: Record<string, unknown>) => void = () => {};
  onClose: () => void = () => {};
  private closed = false;
  private readonly socket: Socket;

  constructor(socket: Socket) {
    this.socket = socket;
    socket.onclose = () => this.close();
    socket.onerror = () => this.close();
  }

  /** Says hello, derives this connection's key, and resolves once it is ready. */
  handshake(secret: Uint8Array, device: string | null, deps: LinkDeps): Promise<void> {
    const nonce = deps.random(32);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        reject(new Error("The Mac didn't answer."));
        this.close();
      }, HANDSHAKE_TIMEOUT);
      this.socket.onmessage = (event) => {
        if (this.cipher) return this.receive(event.data);
        clearTimeout(timer);
        try {
          if (typeof event.data !== "string") throw new Error("bad hello");
          const hello = JSON.parse(event.data);
          if (hello.t !== "hello" || hello.v !== 1) throw new Error("bad hello");
          const mac = fromBase64(hello.nonce);
          if (mac.length !== 32) throw new Error("bad hello");
          this.cipher = new LinkCipher(sessionKey(secret, nonce, mac, device ?? "pair"), PHONE);
          resolve();
        } catch (error) {
          reject(error);
          this.close();
        }
      };
      this.socket.send(JSON.stringify({ t: "hello", v: 1, device, nonce: toBase64(nonce) }));
    });
  }

  send(message: Record<string, unknown>): boolean {
    if (!this.cipher || this.closed) return false;
    const sealed = this.cipher.seal(encoder.encode(JSON.stringify(message)));
    this.socket.send(sealed.buffer.slice(sealed.byteOffset, sealed.byteOffset + sealed.byteLength) as ArrayBuffer);
    return true;
  }

  private receive(data: unknown) {
    if (!(data instanceof ArrayBuffer) || !this.cipher) return this.close();
    try {
      const message = JSON.parse(decoder.decode(this.cipher.open(new Uint8Array(data))));
      if (message && typeof message.type === "string") this.onMessage(message);
    } catch {
      // Forged, replayed or out of order: the link can't be trusted any more.
      this.close();
    }
  }

  close() {
    if (this.closed) return;
    this.closed = true;
    this.cipher = null;
    try {
      this.socket.close();
    } catch {}
    this.onClose();
  }
}

function open(url: string, deps: LinkDeps): Promise<Socket> {
  return new Promise((resolve, reject) => {
    const socket = deps.socket(url);
    socket.binaryType = "arraybuffer";
    const timer = setTimeout(() => {
      socket.onopen = socket.onerror = socket.onclose = null;
      try {
        socket.close();
      } catch {}
      reject(new Error("timeout"));
    }, CONNECT_TIMEOUT);
    socket.onopen = () => {
      clearTimeout(timer);
      resolve(socket);
    };
    socket.onerror = socket.onclose = () => {
      clearTimeout(timer);
      reject(new Error("unreachable"));
    };
  });
}

/** Tries each of the Mac's addresses in turn. */
async function reach(hosts: string[], port: number, deps: LinkDeps): Promise<Socket> {
  for (const host of hosts) {
    try {
      return await open(`ws://${host}:${port}`, deps);
    } catch {}
  }
  throw new Error("Couldn't reach your Mac. Is Astrid open with the phone link on, on the same Wi-Fi?");
}

/** Pairs with a Mac from its QR code: proves the code, gets this phone's key. */
export async function pair(offer: PairingOffer, name: string, deps: LinkDeps): Promise<Pairing> {
  const socket = await reach(offer.hosts, offer.port, deps);
  const connection = new Connection(socket);
  await connection.handshake(offer.secret, null, deps);
  return new Promise((resolve, reject) => {
    connection.onMessage = (message) => {
      if (message.type !== "paired" || typeof message.device !== "string" || typeof message.key !== "string") {
        return reject(new Error("The Mac refused to pair."));
      }
      if (fromBase64(message.key).length !== 32) return reject(new Error("The Mac sent a bad key."));
      resolve({ device: message.device, key: message.key, hosts: offer.hosts, port: offer.port, mac: String(message.name ?? offer.name) });
      connection.close();
    };
    connection.onClose = () => reject(new Error("The code was used or expired. Make a new one on your Mac."));
    connection.send({ type: "pair", name });
  });
}

/**
 * Keeps one connection to a paired Mac, reconnecting with backoff while
 * started. Messages go out only while online; nothing is queued.
 */
export class LinkClient {
  status: LinkStatus = "offline";
  onStatus: (status: LinkStatus, problem?: string) => void = () => {};
  onMessage: (message: Record<string, unknown>) => void = () => {};
  private connection: Connection | null = null;
  private running = false;
  private retry: ReturnType<typeof setTimeout> | null = null;
  private delay = 1000;
  private readonly pairing: Pairing;
  private readonly deps: LinkDeps;

  constructor(pairing: Pairing, deps: LinkDeps) {
    this.pairing = pairing;
    this.deps = deps;
  }

  start() {
    if (this.running) return;
    this.running = true;
    this.connect();
  }

  stop() {
    this.running = false;
    if (this.retry) clearTimeout(this.retry);
    this.retry = null;
    this.connection?.close();
    this.connection = null;
    this.set("offline");
  }

  /** Reconnects now, as when the app comes back to the front. */
  nudge() {
    if (!this.running || this.status !== "offline") return;
    if (this.retry) clearTimeout(this.retry);
    this.retry = null;
    this.delay = 1000;
    this.connect();
  }

  send(message: Record<string, unknown>): boolean {
    return this.status === "online" && !!this.connection?.send(message);
  }

  private set(status: LinkStatus, problem?: string) {
    this.status = status;
    this.onStatus(status, problem);
  }

  private async connect() {
    this.set("connecting");
    try {
      const socket = await reach(this.pairing.hosts, this.pairing.port, this.deps);
      if (!this.running) return socket.close();
      const connection = new Connection(socket);
      connection.onMessage = (message) => this.onMessage(message);
      connection.onClose = () => {
        if (this.connection !== connection) return;
        this.connection = null;
        this.later("The link to your Mac dropped.");
      };
      await connection.handshake(fromBase64(this.pairing.key), this.pairing.device, this.deps);
      if (!this.running) return connection.close();
      this.connection = connection;
      this.delay = 1000;
      // The first sealed frame proves the key; the chat list comes back.
      connection.send({ type: "sync" });
      this.set("online");
    } catch (error) {
      this.later(error instanceof Error ? error.message : "Couldn't reach your Mac.");
    }
  }

  private later(problem: string) {
    this.set("offline", problem);
    if (!this.running) return;
    this.retry = setTimeout(() => {
      this.retry = null;
      if (this.running) this.connect();
    }, this.delay);
    this.delay = Math.min(this.delay * 2, 15000);
  }
}
