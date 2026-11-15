import Foundation

/// ADR 0040: what the user asked Astrid to remember, read from and written to
/// the runtime's `~/Library/Application Support/Astrid/memory.json` (same
/// version 1 format as `src/memory.rs`), so Settings can show and delete it.
public struct MemoryFact: Codable, Equatable, Identifiable, Sendable {
    public var id: UInt32
    public var text: String
    /// Seconds since the Unix epoch.
    public var at: UInt64

    public var date: Date { Date(timeIntervalSince1970: TimeInterval(at)) }
}

public struct MemoryStore: Sendable {
    public let url: URL

    public init(url: URL = MemoryStore.defaultURL) { self.url = url }

    public static var defaultURL: URL {
        if let path = ProcessInfo.processInfo.environment["ASTRID_MEMORY_FILE"] { return URL(fileURLWithPath: path) }
        return FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/Astrid/memory.json")
    }

    private struct File: Codable {
        var version: UInt32
        var next: UInt32
        var facts: [MemoryFact]
    }

    /// Newest first. A missing file is empty; an unreadable one throws, so it
    /// is never overwritten by mistake.
    public func load() throws -> [MemoryFact] {
        try read()?.facts.reversed() ?? []
    }

    private func read() throws -> File? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return try JSONDecoder().decode(File.self, from: data)
    }

    /// Forgets one fact; the rest, and the next id, are kept as they were.
    public func delete(_ id: UInt32) throws {
        guard var file = try read() else { return }
        file.facts.removeAll { $0.id == id }
        try write(file)
    }

    public func deleteAll() throws {
        guard var file = try read() else { return }
        file.facts.removeAll()
        try write(file)
    }

    /// Written whole to a private temp file, then renamed over the old one, as
    /// the runtime does.
    private func write(_ file: File) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(file)
        let tmp = url.appendingPathExtension("tmp")
        FileManager.default.createFile(atPath: tmp.path, contents: data, attributes: [.posixPermissions: 0o600])
        _ = try FileManager.default.replaceItemAt(url, withItemAt: tmp)
        // The swap keeps the old file's mode; memory is always private.
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
    }
}
