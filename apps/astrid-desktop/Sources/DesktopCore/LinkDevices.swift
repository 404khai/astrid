import Foundation

/// A phone paired with this Mac (ADR 0039). Its key is the long-term secret
/// that each of its connections derives a key from. Removing it from the
/// registry revokes it.
public struct PairedDevice: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var name: String
    public var key: Data
    public var paired: Date
    public var lastSeen: Date?
}

public enum LinkLimits {
    /// The phone link's port on this Mac's local network.
    public static let port: UInt16 = 47821
    /// Phones that can be paired at once.
    public static let maxDevices = 8
    /// How long a pairing code works, seconds. It works once.
    public static let pairingLifetime: TimeInterval = 120
    /// The largest frame a phone may send. Asks are short.
    public static let maxInbound = 64 * 1024
    /// The largest frame sent to a phone: a whole chat fits.
    public static let maxOutbound = ChatLimits.maxFileBytes + 64 * 1024
    /// Seconds a connection has to say hello and prove its key.
    public static let handshakeTimeout: TimeInterval = 10
    /// Phones connected at once.
    public static let maxConnections = 4
    /// Characters kept from a phone's name.
    public static let maxName = 40
    /// Characters a phone ask may carry, as the desktop composer keeps.
    public static let maxAsk = 4000
    /// Chats listed on the phone.
    public static let maxListed = 200
    /// Drawings sent to a phone at once (each up to the shape cap).
    public static let maxDiagrams = 40
}

/// The paired phones, kept in a private file next to the chats:
/// `~/Library/Application Support/Astrid/Devices.json`, 0600 in a 0700 folder.
public final class DeviceRegistry {
    public let file: URL
    public private(set) var devices: [PairedDevice]

    private struct Stored: Codable {
        var v: Int
        var devices: [PairedDevice]
    }

    public init(file: URL = DeviceRegistry.defaultFile) {
        self.file = file
        if let data = try? Data(contentsOf: file),
           let stored = try? Self.decoder.decode(Stored.self, from: data), stored.v == 1 {
            devices = stored.devices.filter { $0.key.count == LinkCrypto.secretBytes }
        } else {
            devices = []
        }
    }

    public static var defaultFile: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/Astrid/Devices.json")
    }

    private static let encoder: JSONEncoder = {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.sortedKeys]
        return encoder
    }()

    private static let decoder: JSONDecoder = {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return decoder
    }()

    public func device(_ id: String) -> PairedDevice? { devices.first { $0.id == id } }

    /// Pairs a new phone with a fresh key. The oldest is dropped past the limit.
    public func add(name: String, now: Date = Date()) throws -> PairedDevice {
        let scalars = name.unicodeScalars.filter { !CharacterSet.controlCharacters.contains($0) }
        let cleaned = String(String.UnicodeScalarView(scalars)).trimmingCharacters(in: .whitespaces)
        let device = PairedDevice(
            id: LinkCrypto.random(12).base64URL,
            name: String((cleaned.isEmpty ? "Phone" : cleaned).prefix(LinkLimits.maxName)),
            key: LinkCrypto.random(LinkCrypto.secretBytes),
            paired: now
        )
        var next = devices + [device]
        if next.count > LinkLimits.maxDevices { next.removeFirst(next.count - LinkLimits.maxDevices) }
        try write(next)
        return device
    }

    public func remove(_ id: String) throws {
        try write(devices.filter { $0.id != id })
    }

    /// Notes when a phone last connected; a failed write only loses that note.
    public func touch(_ id: String, now: Date = Date()) {
        guard let index = devices.firstIndex(where: { $0.id == id }) else { return }
        var next = devices
        next[index].lastSeen = now
        try? write(next)
    }

    private func write(_ next: [PairedDevice]) throws {
        let manager = FileManager.default
        try manager.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let data = try Self.encoder.encode(Stored(v: 1, devices: next))
        try data.write(to: file, options: [.atomic])
        try manager.setAttributes([.posixPermissions: 0o600], ofItemAtPath: file.path)
        devices = next
    }
}

/// A pairing code: a one-time secret that works for two minutes, shown as a
/// QR code on the Mac together with where to reach it.
public struct PairingOffer: Equatable {
    public let secret: Data
    public let expires: Date

    public init(now: Date = Date(), secret: Data = LinkCrypto.random(LinkCrypto.secretBytes)) {
        self.secret = secret
        expires = now.addingTimeInterval(LinkLimits.pairingLifetime)
    }

    public func isValid(at now: Date = Date()) -> Bool { now < expires }

    /// The QR's text: `astrid://pair?v=1&h=<addresses>&p=<port>&s=<secret>&n=<Mac name>`.
    public func url(hosts: [String], port: UInt16, name: String) -> String {
        var components = URLComponents()
        components.scheme = "astrid"
        components.host = "pair"
        components.queryItems = [
            URLQueryItem(name: "v", value: String(LinkCrypto.version)),
            URLQueryItem(name: "h", value: hosts.joined(separator: ",")),
            URLQueryItem(name: "p", value: String(port)),
            URLQueryItem(name: "s", value: secret.base64URL),
            URLQueryItem(name: "n", value: String(name.prefix(LinkLimits.maxName))),
        ]
        return components.string ?? ""
    }
}

/// Whether a peer address is on this Mac's own network: loopback, private
/// IPv4, link-local, or IPv6 unique-local. The link refuses anything else.
public enum LocalNetwork {
    public static func isLocal(_ host: String) -> Bool {
        var address = host.lowercased()
        if let percent = address.firstIndex(of: "%") { address = String(address[..<percent]) }
        if address.hasPrefix("::ffff:") { address = String(address.dropFirst(7)) }
        let octets = address.split(separator: ".", omittingEmptySubsequences: false).compactMap { UInt8($0) }
        if octets.count == 4, address.split(separator: ".").count == 4 {
            switch (octets[0], octets[1]) {
            case (127, _), (10, _), (192, 168), (169, 254): return true
            case (172, let b): return (16...31).contains(b)
            default: return false
            }
        }
        guard address.contains(":") else { return false }
        if address == "::1" { return true }
        return address.hasPrefix("fe8") || address.hasPrefix("fe9") || address.hasPrefix("fea") || address.hasPrefix("feb")
            || address.hasPrefix("fc") || address.hasPrefix("fd")
    }
}
