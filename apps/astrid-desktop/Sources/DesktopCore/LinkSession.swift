import Foundation

/// One phone connection's handshake and sealing (ADR 0039), apart from the
/// socket so it can be tested headlessly.
///
/// 1. The phone sends a text hello: `{"t":"hello","v":1,"device":ID|null,"nonce":B64}`.
///    `null` means it is pairing, which needs a live pairing code.
/// 2. The Mac answers `{"t":"hello","v":1,"nonce":B64}`. From here on, frames
///    are binary and sealed (`LinkCipher`), and text frames end the connection.
/// 3. The phone's first sealed frame proves it holds the key.
///    - Pairing: it is `{"type":"pair","name":…}`. The code is used up, the Mac
///      replies `{"type":"paired","device":…,"key":…,"name":…}` and closes, and
///      the phone reconnects as that device.
///    - Paired: the frame is the phone's first request, and the link is open.
///
/// Anything else, including a frame that fails to open, closes the
/// connection without saying why.
public final class LinkSession {
    public enum Frame: Equatable {
        case text(String)
        case binary(Data)
    }

    public enum Closing: Equatable {
        case protocolError(String)
        case unknownDevice
        case notPairing
        case unauthentic
        case paired(PairedDevice)
    }

    private enum State {
        case hello
        case proving(device: PairedDevice?)
        case open(device: PairedDevice)
        case closed
    }

    private var state = State.hello
    private var cipher: LinkCipher?
    private let registry: DeviceRegistry
    private let macName: String
    /// Uses up the pairing code if one is live, and says whether there was one.
    private let consumeOffer: (_ peek: Bool) -> Data?

    /// Frames to write to the socket.
    public var write: (Frame) -> Void = { _ in }
    /// Called once, when the connection should close.
    public var onClose: (Closing) -> Void = { _ in }
    /// Each request from a paired phone, after the handshake.
    public var onMessage: ([String: Any]) -> Void = { _ in }
    /// Called once the phone has proved its key.
    public var onOpen: (PairedDevice) -> Void = { _ in }

    /// - Parameter offer: with `peek` true, the live pairing code's secret
    ///   without using it up; with `peek` false, uses it up and returns it,
    ///   or nil if it expired or was already used.
    public init(registry: DeviceRegistry, macName: String, offer: @escaping (_ peek: Bool) -> Data?) {
        self.registry = registry
        self.macName = macName
        consumeOffer = offer
    }

    public var device: PairedDevice? {
        if case .open(let device) = state { return device }
        return nil
    }

    public var isClosed: Bool {
        if case .closed = state { return true }
        return false
    }

    public func receive(_ frame: Frame) {
        switch (state, frame) {
        case (.closed, _):
            return
        case (.hello, .text(let text)):
            hello(text)
        case (.proving(let device), .binary(let data)):
            guard let message = open(data) else { return }
            if let device {
                state = .open(device: device)
                registry.touch(device.id)
                onOpen(device)
                onMessage(message)
            } else {
                pair(message)
            }
        case (.open, .binary(let data)):
            guard let message = open(data) else { return }
            onMessage(message)
        default:
            close(.protocolError("unexpected frame"))
        }
    }

    /// Seals one message to the phone; false once the link isn't open.
    @discardableResult
    public func send(_ message: [String: Any]) -> Bool {
        guard case .open = state else { return false }
        return seal(message)
    }

    public func close(_ reason: Closing) {
        if case .closed = state { return }
        state = .closed
        cipher = nil
        onClose(reason)
    }

    private func hello(_ text: String) {
        guard text.utf8.count <= 1024,
              let object = try? JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any],
              object["t"] as? String == "hello", object["v"] as? Int == LinkCrypto.version,
              let nonceText = object["nonce"] as? String, let phoneNonce = Data(base64Encoded: nonceText),
              phoneNonce.count == LinkCrypto.nonceBytes else {
            return close(.protocolError("bad hello"))
        }
        let secret: Data
        let label: String
        var device: PairedDevice?
        if let id = object["device"] as? String {
            guard let known = registry.device(id) else { return close(.unknownDevice) }
            device = known
            secret = known.key
            label = id
        } else {
            guard let live = consumeOffer(true) else { return close(.notPairing) }
            secret = live
            label = "pair"
        }
        let macNonce = LinkCrypto.random(LinkCrypto.nonceBytes)
        let key = LinkCrypto.sessionKey(secret: secret, phoneNonce: phoneNonce, macNonce: macNonce, label: label)
        cipher = LinkCipher(key: key, sends: .mac)
        state = .proving(device: device)
        write(.text(Self.json(["t": "hello", "v": LinkCrypto.version, "nonce": macNonce.base64EncodedString()])))
    }

    private func pair(_ message: [String: Any]) {
        guard message["type"] as? String == "pair" else { return close(.protocolError("expected pair")) }
        // The code works once: a second phone that saw it gets nothing.
        guard consumeOffer(false) != nil else { return close(.notPairing) }
        guard let device = try? registry.add(name: message["name"] as? String ?? "Phone") else {
            return close(.protocolError("could not save the phone"))
        }
        _ = seal(["type": "paired", "device": device.id, "key": device.key.base64EncodedString(), "name": macName])
        close(.paired(device))
    }

    private func open(_ data: Data) -> [String: Any]? {
        guard data.count <= LinkLimits.maxInbound + 16 else {
            close(.protocolError("frame too large"))
            return nil
        }
        guard var cipher, let plaintext = try? cipher.open(data) else {
            close(.unauthentic)
            return nil
        }
        self.cipher = cipher
        guard let message = try? JSONSerialization.jsonObject(with: plaintext) as? [String: Any], message["type"] is String else {
            close(.protocolError("bad message"))
            return nil
        }
        return message
    }

    private func seal(_ message: [String: Any]) -> Bool {
        guard var cipher, let data = try? JSONSerialization.data(withJSONObject: message),
              data.count <= LinkLimits.maxOutbound, let sealed = try? cipher.seal(data) else { return false }
        self.cipher = cipher
        write(.binary(sealed))
        return true
    }

    static func json(_ object: [String: Any]) -> String {
        guard let data = try? JSONSerialization.data(withJSONObject: object, options: [.sortedKeys]) else { return "{}" }
        return String(decoding: data, as: UTF8.self)
    }
}
