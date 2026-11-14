// Run: node src/link/crypto.test.mjs. The vector is the Mac's
// (apps/astrid-desktop/Tests/DesktopCoreTests/PhoneLinkTests.swift).
import assert from "node:assert/strict";
import { LinkCipher, MAC, PHONE, fromBase64, hex, sessionKey, toBase64 } from "./crypto.ts";
import { parsePairing } from "./pairing.ts";

const secret = Uint8Array.from({ length: 32 }, (_, i) => i);
const phone = Uint8Array.from({ length: 32 }, (_, i) => 0x40 + i);
const mac = Uint8Array.from({ length: 32 }, (_, i) => 0x80 + i);
const key = sessionKey(secret, phone, mac, "pair");
assert.equal(hex(key), "913dde7addb42a6c328279b708c3b1293ba3eeecb2cb78b60c76e8bcde8cc6eb");

// The Mac's first frame opens on the phone.
const fromMac = new LinkCipher(key, MAC).seal(new TextEncoder().encode('{"type":"pong"}'));
assert.equal(hex(fromMac), "5ddd20fb7dbeb0d45560bb529576d0fdf2b085a740514d48855b2af09c57fb");
const onPhone = new LinkCipher(key, PHONE);
assert.equal(new TextDecoder().decode(onPhone.open(fromMac)), '{"type":"pong"}');
assert.throws(() => onPhone.open(fromMac), "replay");

// Out of order, tampered and reflected frames fail.
const sender = new LinkCipher(key, MAC);
const one = sender.seal(new Uint8Array([1]));
const two = sender.seal(new Uint8Array([2]));
const receiver = new LinkCipher(key, PHONE);
assert.throws(() => receiver.open(two));
const bad = one.slice();
bad[0] ^= 1;
assert.throws(() => receiver.open(bad));
assert.deepEqual(receiver.open(one), new Uint8Array([1]));
const mine = new LinkCipher(key, PHONE).seal(new Uint8Array([9]));
assert.throws(() => new LinkCipher(key, PHONE).open(mine));

assert.deepEqual(fromBase64(toBase64(secret)), secret);
assert.deepEqual(fromBase64("-_8"), Uint8Array.from([0xfb, 0xff]));

// The Mac's QR text.
const offer = parsePairing("astrid://pair?v=1&h=192.168.1.4,10.0.0.3&p=47821&s=" + "-".repeat(43) + "&n=Khai's%20Mac");
assert.deepEqual(offer?.hosts, ["192.168.1.4", "10.0.0.3"]);
assert.equal(offer?.port, 47821);
assert.equal(offer?.name, "Khai's Mac");
assert.equal(offer?.secret.length, 32);
assert.equal(parsePairing("astrid://pair?v=2&h=1.2.3.4&p=1&s=AA"), null);
assert.equal(parsePairing("https://example.com"), null);
assert.equal(parsePairing("astrid://pair?v=1&h=8.8.8.8&p=47821&s=" + "A".repeat(43)), null, "public address");
console.log("link crypto: ok");
