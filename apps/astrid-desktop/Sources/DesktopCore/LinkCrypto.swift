import CryptoKit
import Foundation

/// The phone link's sealing (ADR 0039). Each connection has its own key, made
/// with HKDF-SHA256 from a long-term secret (the pairing code's, or the paired
/// phone's) and a random nonce from each side. Every frame after the hello is
/// AES-256-GCM under that key. The 12-byte nonce is the sender's direction,
/// then a counter that counts up from 0. The receiver expects the next count
/// exactly, so a frame that is replayed, dropped or reordered ends the
/// connection. Only the ciphertext and the 16-byte tag go on the wire.
public enum LinkCrypto {
    public static let version = 1
    public static let nonceBytes = 32
    public static let secretBytes = 32
    /// Who sent a frame: the first four bytes of its nonce.
    public enum Direction: UInt32 { case phone = 0, mac = 1 }

    /// This connection's key, from the long-term `secret` and both hello nonces.
    /// `label` is the phone's device ID, or "pair" when pairing.
    public static func sessionKey(secret: Data, phoneNonce: Data, macNonce: Data, label: String) -> SymmetricKey {
        HKDF<SHA256>.deriveKey(
            inputKeyMaterial: SymmetricKey(data: secret),
            salt: phoneNonce + macNonce,
            info: Data("astrid-link/1 \(label)".utf8),
            outputByteCount: 32
        )
    }

    public static func nonce(_ direction: Direction, _ counter: UInt64) -> Data {
        var data = Data()
        withUnsafeBytes(of: direction.rawValue.bigEndian) { data.append(contentsOf: $0) }
        withUnsafeBytes(of: counter.bigEndian) { data.append(contentsOf: $0) }
        return data
    }

    public static func random(_ count: Int) -> Data {
        var generator = SystemRandomNumberGenerator()
        return Data((0..<count).map { _ in UInt8.random(in: .min ... .max, using: &generator) })
    }
}

public enum LinkCryptoError: Error, Equatable {
    case short
    case unauthentic
}

/// One connection's two counters, in one direction each.
public struct LinkCipher {
    private let key: SymmetricKey
    private let sends: LinkCrypto.Direction
    private let receives: LinkCrypto.Direction
    public private(set) var sent: UInt64 = 0
    public private(set) var received: UInt64 = 0

    public init(key: SymmetricKey, sends: LinkCrypto.Direction) {
        self.key = key
        self.sends = sends
        receives = sends == .mac ? .phone : .mac
    }

    public mutating func seal(_ plaintext: Data) throws -> Data {
        let nonce = try AES.GCM.Nonce(data: LinkCrypto.nonce(sends, sent))
        let box = try AES.GCM.seal(plaintext, using: key, nonce: nonce)
        sent += 1
        return Data(box.ciphertext) + box.tag
    }

    public mutating func open(_ frame: Data) throws -> Data {
        guard frame.count >= 16 else { throw LinkCryptoError.short }
        let frame = Data(frame)
        let nonce = try AES.GCM.Nonce(data: LinkCrypto.nonce(receives, received))
        let box = try AES.GCM.SealedBox(nonce: nonce, ciphertext: frame.prefix(frame.count - 16), tag: frame.suffix(16))
        guard let plaintext = try? AES.GCM.open(box, using: key) else { throw LinkCryptoError.unauthentic }
        received += 1
        return plaintext
    }
}

extension Data {
    /// URL-safe base64 without padding, as the pairing code carries its secret.
    public var base64URL: String {
        base64EncodedString().replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: "=", with: "")
    }

    public init?(base64URL: String) {
        var text = base64URL.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/")
        while text.count % 4 != 0 { text += "=" }
        self.init(base64Encoded: text)
    }
}
