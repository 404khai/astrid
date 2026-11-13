import Foundation

/// ADR 0029: timers and alarms owned by the resident presence host. Pure
/// state with explicit limits; the host persists and fires it.
public struct TimerEntry: Codable, Equatable, Sendable {
    public enum Kind: String, Codable, Sendable { case timer, alarm }
    public var id: String
    public var kind: Kind
    public var label: String
    public var fireAt: Date
    public var createdAt: Date

    public init(id: String, kind: Kind, label: String, fireAt: Date, createdAt: Date) {
        (self.id, self.kind, self.label, self.fireAt, self.createdAt) = (id, kind, label, fireAt, createdAt)
    }
}

public struct TimerStore: Codable, Equatable, Sendable {
    public static let maxActive = 32
    public static let maxTimerSeconds = 24 * 60 * 60
    public static let maxAlarmAhead: TimeInterval = 7 * 24 * 60 * 60
    public static let maxLabel = 120

    public private(set) var entries: [TimerEntry] = []
    public var version = 1

    public init() {}

    public mutating func startTimer(id: String, seconds: Int, label: String, now: Date) throws -> TimerEntry {
        guard (1...Self.maxTimerSeconds).contains(seconds) else {
            throw PresenceProtocolError("timer must be 1 second to 24 hours")
        }
        return try add(TimerEntry(id: id, kind: .timer, label: label, fireAt: now.addingTimeInterval(TimeInterval(seconds)), createdAt: now))
    }

    public mutating func setAlarm(id: String, at: Date, label: String, now: Date) throws -> TimerEntry {
        guard at > now, at.timeIntervalSince(now) <= Self.maxAlarmAhead else {
            throw PresenceProtocolError("alarm must be in the future and within 7 days")
        }
        return try add(TimerEntry(id: id, kind: .alarm, label: label, fireAt: at, createdAt: now))
    }

    private mutating func add(_ entry: TimerEntry) throws -> TimerEntry {
        guard PresenceProtocol.isRequestId(entry.id) else { throw PresenceProtocolError("timer id must be 1...64 of [A-Za-z0-9-]") }
        let label = entry.label.trimmingCharacters(in: .whitespacesAndNewlines)
        guard label.count <= Self.maxLabel, !label.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }) else {
            throw PresenceProtocolError("label must be at most \(Self.maxLabel) printable characters")
        }
        guard !entries.contains(where: { $0.id == entry.id }) else { throw PresenceProtocolError("timer \(entry.id) already exists") }
        guard entries.count < Self.maxActive else { throw PresenceProtocolError("at most \(Self.maxActive) active timers") }
        var entry = entry
        entry.label = label
        entries.append(entry)
        entries.sort { $0.fireAt < $1.fireAt }
        return entry
    }

    @discardableResult
    public mutating func cancel(id: String) -> TimerEntry? {
        guard let index = entries.firstIndex(where: { $0.id == id }) else { return nil }
        return entries.remove(at: index)
    }

    /// Removes and returns entries due at `now`, oldest first. Each fires once.
    public mutating func takeDue(now: Date) -> [TimerEntry] {
        let due = entries.filter { $0.fireAt <= now }
        entries.removeAll { $0.fireAt <= now }
        return due
    }

    // MARK: Persistence

    public func encoded() throws -> Data {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.sortedKeys, .prettyPrinted]
        return try encoder.encode(self)
    }

    /// Decodes and re-validates; anything malformed or over the limits is dropped.
    public static func decode(_ data: Data) -> TimerStore {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        guard let raw = try? decoder.decode(TimerStore.self, from: data), raw.version == 1 else { return TimerStore() }
        var store = TimerStore()
        for entry in raw.entries.prefix(maxActive) where PresenceProtocol.isRequestId(entry.id) && entry.label.count <= maxLabel {
            if !store.entries.contains(where: { $0.id == entry.id }) { store.entries.append(entry) }
        }
        store.entries.sort { $0.fireAt < $1.fireAt }
        return store
    }

    /// Writes atomically with owner-only permissions.
    public func save(to url: URL) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        try encoded().write(to: url, options: [.atomic])
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
    }

    public static func load(from url: URL) -> TimerStore {
        guard let data = try? Data(contentsOf: url), data.count <= 256 * 1024 else { return TimerStore() }
        return decode(data)
    }

    public static var defaultURL: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("Astrid/timers.json")
    }
}

/// Human wording for a fired timer, e.g. "Timer done: tea (late 3 min)".
public func timerFiredText(_ entry: TimerEntry, now: Date) -> String {
    let late = now.timeIntervalSince(entry.fireAt)
    let base = entry.kind == .alarm ? "Alarm" : "Timer done"
    let label = entry.label.isEmpty ? "" : ": \(entry.label)"
    let lateText = late >= 60 ? " (late \(Int(late / 60)) min)" : ""
    return base + label + lateText
}
