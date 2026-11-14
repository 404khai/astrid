import { fromBase64 } from "./crypto.ts";

/** A Mac's pairing QR (ADR 0039):
 * `astrid://pair?v=1&h=<addresses>&p=<port>&s=<secret>&n=<Mac name>`. */
export type PairingOffer = { hosts: string[]; port: number; secret: Uint8Array; name: string };

/** Private, link-local and loopback addresses only, as the Mac accepts. */
export function isLocal(host: string): boolean {
  const parts = host.split(".");
  if (parts.length === 4 && parts.every((p) => /^\d{1,3}$/.test(p) && Number(p) < 256)) {
    const [a, b] = parts.map(Number);
    return a === 10 || a === 127 || (a === 192 && b === 168) || (a === 169 && b === 254) || (a === 172 && b >= 16 && b <= 31);
  }
  return false;
}

export function parsePairing(text: string): PairingOffer | null {
  const match = /^astrid:\/\/pair\?(.*)$/.exec(text.trim());
  if (!match) return null;
  const query = new Map<string, string>();
  for (const pair of match[1].split("&")) {
    const at = pair.indexOf("=");
    if (at > 0) query.set(pair.slice(0, at), decodeURIComponent(pair.slice(at + 1).replace(/\+/g, " ")));
  }
  if (query.get("v") !== "1") return null;
  const hosts = (query.get("h") ?? "").split(",").filter(isLocal).slice(0, 4);
  const port = Number(query.get("p"));
  let secret: Uint8Array;
  try {
    secret = fromBase64(query.get("s") ?? "");
  } catch {
    return null;
  }
  if (!hosts.length || !Number.isInteger(port) || port < 1 || port > 65535 || secret.length !== 32) return null;
  return { hosts, port, secret, name: (query.get("n") ?? "Mac").slice(0, 40) };
}
