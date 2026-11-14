import Foundation

/// Desktop chats on disk (ADR 0037): one JSON file per chat in a private
/// folder, `~/Library/Application Support/Astrid/Chats/<id>.json`. Writes are
/// atomic, the folder is 0700 and the files are 0600.
public final class ChatStore {
    public let directory: URL

    public init(directory: URL = ChatStore.defaultDirectory) {
        self.directory = directory
    }

    public static var defaultDirectory: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/Astrid/Chats", isDirectory: true)
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

    private func file(for id: UUID) -> URL {
        directory.appendingPathComponent("\(id.uuidString.lowercased()).json")
    }

    /// All readable chats, most recent activity first. Files that are too
    /// large, unreadable, from a newer version or named for another id are
    /// skipped, never rewritten.
    public func loadAll() -> [ChatSession] {
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: directory.path) else { return [] }
        var chats: [ChatSession] = []
        for name in names where name.hasSuffix(".json") {
            let url = directory.appendingPathComponent(name)
            guard let size = (try? FileManager.default.attributesOfItem(atPath: url.path))?[.size] as? Int,
                  size <= ChatLimits.maxFileBytes,
                  let data = try? Data(contentsOf: url),
                  let chat = try? Self.decoder.decode(ChatSession.self, from: data),
                  chat.v <= ChatSession.currentVersion,
                  name == "\(chat.id.uuidString.lowercased()).json" else { continue }
            chats.append(chat)
        }
        return Array(Self.sorted(chats).prefix(ChatLimits.maxChats))
    }

    /// Most recent activity first; ties by creation, then id, so the order is stable.
    public static func sorted(_ chats: [ChatSession]) -> [ChatSession] {
        chats.sorted {
            if $0.updated != $1.updated { return $0.updated > $1.updated }
            if $0.created != $1.created { return $0.created > $1.created }
            return $0.id.uuidString < $1.id.uuidString
        }
    }

    /// A chat as it is saved: the phone link sends this same shape (ADR 0039).
    public static func encode(_ chat: ChatSession) throws -> Data {
        try encoder.encode(chat)
    }

    public static func decode(_ data: Data) throws -> ChatSession {
        try decoder.decode(ChatSession.self, from: data)
    }

    public func save(_ chat: ChatSession) throws {
        let manager = FileManager.default
        try manager.createDirectory(at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let data = try Self.encoder.encode(chat)
        let url = file(for: chat.id)
        try data.write(to: url, options: [.atomic])
        try manager.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
    }
}
