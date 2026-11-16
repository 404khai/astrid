import Foundation

/// Goblin group chats (ADR 0037 amendment 14): a persisted thread per group,
/// @mentions, reactions, and the turn plan that decides who speaks next.
/// Everything here is pure or file-backed so it can be tested without a model.

/// Who wrote a message: the user, Astrid (the group's overseer) or a goblin by id.
public enum GroupAuthor: Equatable, Hashable, Sendable {
    case you, astrid, goblin(String)

    public var key: String {
        switch self {
        case .you: "you"
        case .astrid: "astrid"
        case .goblin(let id): id
        }
    }

    public init(key: String) {
        switch key {
        case "you": self = .you
        case "astrid": self = .astrid
        default: self = .goblin(key)
        }
    }
}

public struct GroupMessage: Codable, Equatable, Identifiable, Sendable {
    public var id: UUID
    /// `GroupAuthor.key`.
    public var author: String
    public var text: String
    public var at: Date
    /// The goblin expression it was said with (an id from the Goblins library).
    public var mood: String?
    /// Who was tagged, by author key.
    public var mentions: [String]
    /// Emoji → who reacted with it, by author key, in order.
    public var reactions: [String: [String]]
    /// Set when the reply failed or was stopped.
    public var outcome: String?
    /// An inline chart the goblin drew (`GroupChart`), absent in older threads.
    public var chart: JSONValue?

    public static let maxText = 4000

    public init(id: UUID = UUID(), author: GroupAuthor, text: String, at: Date = Date(), mood: String? = nil, mentions: [String] = [], reactions: [String: [String]] = [:], outcome: String? = nil, chart: JSONValue? = nil) {
        self.id = id
        self.author = author.key
        self.text = String(text.prefix(Self.maxText))
        // Whole seconds, as the file keeps them.
        self.at = Date(timeIntervalSince1970: at.timeIntervalSince1970.rounded(.down))
        self.mood = mood
        self.mentions = mentions
        self.reactions = reactions
        self.outcome = outcome
        self.chart = chart
    }

    public var from: GroupAuthor { GroupAuthor(key: author) }

    /// Adds `who`'s reaction, or takes it back if it is already there.
    /// Only a few emoji per message, and one of each per author.
    public mutating func toggle(_ emoji: String, by who: GroupAuthor) {
        let emoji = String(emoji.prefix(8))
        guard !emoji.isEmpty else { return }
        var list = reactions[emoji] ?? []
        if let i = list.firstIndex(of: who.key) {
            list.remove(at: i)
        } else {
            guard reactions[emoji] != nil || reactions.count < GroupReactions.maxPerMessage else { return }
            list.append(who.key)
        }
        reactions[emoji] = list.isEmpty ? nil : list
    }
}

public enum GroupReactions {
    public static let maxPerMessage = 6
    /// The quick picks offered on a message.
    public static let quick = ["👍", "❤️", "😂", "🎉", "👀", "🤔"]
}

public struct GroupThread: Codable, Equatable, Sendable {
    public var version = 1
    public var group: UUID
    public var messages: [GroupMessage]

    public static let maxMessages = 500

    public init(group: UUID, messages: [GroupMessage] = []) {
        self.group = group
        self.messages = messages
    }

    public mutating func append(_ message: GroupMessage) {
        messages.append(message)
        if messages.count > Self.maxMessages { messages.removeFirst(messages.count - Self.maxMessages) }
    }
}

/// One JSON file per group in `Groups/`, private, written atomically.
public struct GroupThreadStore: Sendable {
    public let directory: URL

    public init(directory: URL) { self.directory = directory }

    private func url(_ group: UUID) -> URL { directory.appendingPathComponent("\(group.uuidString).json") }

    /// A missing or unreadable file is an empty thread.
    public func load(_ group: UUID) -> GroupThread {
        guard let data = try? Data(contentsOf: url(group)),
              let thread = try? Self.decoder.decode(GroupThread.self, from: data), thread.group == group else {
            return GroupThread(group: group)
        }
        return thread
    }

    public func save(_ thread: GroupThread) throws {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let data = try Self.encoder.encode(thread)
        let target = url(thread.group)
        try data.write(to: target, options: .atomic)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: target.path)
    }

    public func delete(_ group: UUID) { try? FileManager.default.removeItem(at: url(group)) }

    static let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        e.outputFormatting = [.sortedKeys]
        return e
    }()
    static let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.dateDecodingStrategy = .iso8601
        return d
    }()
}

/// A group member as the plan and prompts see it.
public struct GroupMember: Equatable, Sendable {
    public var id: String
    public var name: String
    public init(id: String, name: String) {
        self.id = id
        self.name = name
    }
}

public enum Mentions {
    /// `@all` (or `@everyone`) tags every goblin in the group.
    public static let everyone = ["all", "everyone"]

    /// Who `text` tags, by author key, in the order first tagged: `@Name`
    /// (case-insensitive, whole name, longest names first) or `@astrid`.
    public static func find(in text: String, members: [GroupMember]) -> [String] {
        let lower = text.lowercased()
        var found: [(at: String.Index, key: String)] = []
        let candidates = members.map { ($0.name.lowercased(), $0.id) } + [("astrid", "astrid")] + everyone.map { ($0, "@all") }
        var taken: [Range<String.Index>] = []
        for (name, key) in candidates.sorted(by: { $0.0.count > $1.0.count }) where !name.isEmpty {
            var from = lower.startIndex
            while let range = lower.range(of: "@" + name, range: from..<lower.endIndex) {
                from = range.upperBound
                let after = range.upperBound < lower.endIndex ? lower[range.upperBound] : " "
                let before = range.lowerBound > lower.startIndex ? lower[lower.index(before: range.lowerBound)] : " "
                guard !(after.isLetter || after.isNumber), !(before.isLetter || before.isNumber), !taken.contains(where: { $0.overlaps(range) }) else { continue }
                taken.append(range)
                if !found.contains(where: { $0.key == key }) { found.append((range.lowerBound, key)) }
            }
        }
        var keys: [String] = []
        for key in found.sorted(by: { $0.at < $1.at }).map(\.key) {
            for k in key == "@all" ? members.map(\.id) : [key] where !keys.contains(k) { keys.append(k) }
        }
        return keys
    }
}

/// Who speaks next. A user message goes to whoever it tags, or to Astrid if
/// nobody is tagged. Each reply's tags queue those members after it. A round
/// (everything after one user message) is bounded so goblins can't tag each
/// other forever.
public struct GroupTurnPlan: Equatable, Sendable {
    public static let maxTurns = 6
    public static let maxPerSpeaker = 2

    /// This round's turn budget: 6, or enough for everyone the user tagged
    /// (as with @all) plus two follow-ups.
    public private(set) var limit = maxTurns
    public private(set) var queue: [String] = []
    public private(set) var spoken: [String: Int] = [:]
    public private(set) var turns = 0

    public init() {}

    /// Starts a round for the user's message.
    public mutating func start(mentions: [String]) {
        queue = []
        spoken = [:]
        turns = 0
        enqueue(mentions.isEmpty ? ["astrid"] : mentions, after: nil)
        limit = max(Self.maxTurns, queue.count + 2)
    }

    /// After `speaker` replies tagging `mentions`.
    public mutating func replied(_ speaker: String, mentions: [String]) {
        enqueue(mentions, after: speaker)
    }

    /// The next speaker, or nil when the round is over.
    public mutating func next() -> String? {
        guard turns < limit, !queue.isEmpty else {
            queue = []
            return nil
        }
        let speaker = queue.removeFirst()
        turns += 1
        spoken[speaker, default: 0] += 1
        return speaker
    }

    public mutating func stop() { queue = [] }

    private mutating func enqueue(_ keys: [String], after speaker: String?) {
        for key in keys where key != "you" && key != speaker && !queue.contains(key) && spoken[key, default: 0] < Self.maxPerSpeaker {
            queue.append(key)
        }
    }
}

/// A goblin's (or Astrid's) reply, split from its trailing ```group block:
/// `{"mood": "happy", "react": "👍", "remember": "…"}`. The block is never shown.
public struct GroupReply: Equatable, Sendable {
    public var text: String
    public var mood: String?
    public var react: String?
    public var remember: String?
    public var chart: JSONValue?

    public static let moods: Set<String> = ["idle", "happy", "talking", "thinking", "wink", "surprised", "sad", "sleeping", "searching", "laughing", "confused", "celebrate", "angry"]

    public static func parse(_ raw: String) -> GroupReply {
        var text = raw
        var meta: [String: Any] = [:]
        if let open = raw.range(of: "```group", options: .backwards) {
            let rest = raw[open.upperBound...]
            let body = rest.range(of: "```").map { rest[..<$0.lowerBound] } ?? rest
            meta = (try? JSONSerialization.jsonObject(with: Data(body.utf8)) as? [String: Any]) ?? [:]
            text = String(raw[..<open.lowerBound])
        }
        text = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let mood = (meta["mood"] as? String).flatMap { moods.contains($0) ? $0 : nil }
        let react = (meta["react"] as? String).flatMap { $0.isEmpty || $0.count > 2 ? nil : String($0.prefix(8)) }
        let remember = (meta["remember"] as? String).flatMap { s -> String? in
            let s = s.trimmingCharacters(in: .whitespacesAndNewlines)
            return s.isEmpty ? nil : String(s.prefix(GoblinMemory.maxFact))
        }
        let chart = GroupChart.check(meta["chart"])
        return GroupReply(text: text, mood: mood, react: react, remember: remember, chart: chart)
    }

    /// What to show while streaming: everything before the trailing block.
    public static func visible(_ streamed: String) -> String {
        (streamed.components(separatedBy: "```").first ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

/// What a goblin remembers, across its groups: a short list of facts in
/// `Goblins/Memory/<id>.json` (private). A goblin adds to it with `remember`;
/// the user can read and clear it in the group panel.
public struct GoblinMemory: Codable, Equatable, Sendable {
    public var version = 1
    public var facts: [Fact]

    public struct Fact: Codable, Equatable, Sendable {
        public var text: String
        public var at: Date
        public init(text: String, at: Date = Date()) {
            self.text = text
            self.at = Date(timeIntervalSince1970: at.timeIntervalSince1970.rounded(.down))
        }
    }

    public static let maxFacts = 40
    public static let maxFact = 240

    public init(facts: [Fact] = []) { self.facts = facts }

    /// Adds a fact (ignoring one already there), dropping the oldest past the limit.
    public mutating func remember(_ text: String, at: Date = Date()) {
        let text = String(text.trimmingCharacters(in: .whitespacesAndNewlines).prefix(Self.maxFact))
        guard !text.isEmpty, !facts.contains(where: { $0.text.caseInsensitiveCompare(text) == .orderedSame }) else { return }
        facts.append(Fact(text: text, at: at))
        if facts.count > Self.maxFacts { facts.removeFirst(facts.count - Self.maxFacts) }
    }
}

public struct GoblinMemoryStore: Sendable {
    public let directory: URL
    public init(directory: URL) { self.directory = directory }

    private func url(_ goblin: String) -> URL {
        let slug = goblin.map { $0.isLetter || $0.isNumber || $0 == "-" || $0 == "." ? $0 : "_" }
        return directory.appendingPathComponent(String(slug) + ".json")
    }

    public func load(_ goblin: String) -> GoblinMemory {
        guard let data = try? Data(contentsOf: url(goblin)),
              let memory = try? GroupThreadStore.decoder.decode(GoblinMemory.self, from: data) else { return GoblinMemory() }
        return memory
    }

    public func save(_ memory: GoblinMemory, for goblin: String) throws {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let target = url(goblin)
        try GroupThreadStore.encoder.encode(memory).write(to: target, options: .atomic)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: target.path)
    }
}

/// The ask sent for one turn. The group's name and description are its
/// purpose; the speaker's role and memory come next, then the recent thread
/// (newest last, bounded) and who handed the turn over.
public enum GroupPrompt {
    public static let maxTranscript = 6000

    public static func build(group: GoblinGroup, speaker: String, members: [GroupMember], thread: [GroupMessage], memory: GoblinMemory, handoff: GroupMessage?) -> String {
        let name = { (key: String) -> String in
            switch GroupAuthor(key: key) {
            case .you: return "User"
            case .astrid: return "Astrid"
            case .goblin(let id): return members.first { $0.id == id }?.name ?? "Goblin"
            }
        }
        let me = name(speaker)
        var out: [String] = []
        out.append("You are \(me), in a group chat called \"\(group.name)\".")
        if let about = group.about, !about.isEmpty { out.append("What this group is for: \(about)") }
        if speaker == "astrid" {
            out.append("You are Astrid, the group's overseer: you keep the goblins on task, answer the user, check work and tag the goblin best suited (@Name) when someone else should pick something up.")
        } else if let role = group.role(of: speaker) {
            out.append("Your role here: \(role)")
        }
        let roster = members.map { m in group.role(of: m.id).map { "@\(m.name) (\($0))" } ?? "@\(m.name)" }
        out.append("Members: " + (["@astrid (overseer)"] + roster.filter { !$0.hasPrefix("@\(me) ") && $0 != "@\(me)" }).joined(separator: ", ") + ", and the user.")
        if !memory.facts.isEmpty {
            out.append("What you remember:\n" + memory.facts.suffix(12).map { "- \($0.text)" }.joined(separator: "\n"))
        }
        var lines: [String] = []
        var used = 0
        for message in thread.reversed() {
            let line = "\(name(message.author)): \(message.text)"
            used += line.count
            if used > maxTranscript { break }
            lines.insert(line, at: 0)
        }
        if !lines.isEmpty { out.append("The chat so far:\n" + lines.joined(separator: "\n")) }
        if let handoff {
            out.append("\(name(handoff.author)) tagged you: \"\(handoff.text)\"")
        }
        out.append("""
        Reply as \(me) in one short chat message (a few sentences, no headings). Tag another member with @Name only to hand them something specific. \
        Then end with a fenced ```group block holding JSON: "mood" (one of happy, talking, thinking, wink, surprised, sad, laughing, confused, celebrate, searching), \
        optionally "react" (one emoji for the message you're answering), optionally "remember" (one short fact about your role or this group worth keeping) \
        and optionally "chart" when numbers are clearer drawn. \(GroupChart.guide)
        """)
        return out.joined(separator: "\n\n")
    }
}

/// An inline chart in a goblin's reply, drawn by the group page with Bklit.
/// Checked here (kind, sizes, finite numbers) and again by the page.
public enum GroupChart {
    public static let kinds: Set<String> = ["line", "area", "bar", "sparkline", "pie", "ring", "funnel", "gauge", "radar"]
    public static let maxBytes = 8 * 1024

    /// How the model picks a chart, for the prompt.
    public static let guide = """
    Pick the chart that fits: "line" or "area" for a trend over dates ({"kind","title","labels":[ISO dates],"series":[{"name","values":[numbers]}]}); \
    "bar" to compare categories (same shape, labels are names); "sparkline" for a tiny trend (one series); \
    "pie" for parts of a whole and "ring" for progress per item ({"kind","title","items":[{"label","value","max"?}]}); \
    "funnel" for stages that narrow (items); "gauge" for one score out of 100 ({"kind","title","value","label"}); \
    "radar" to rate a few things 0-100 on the same axes ({"kind","title","axes":[names],"series":[...]}). At most one chart, only with real numbers.
    """

    public static func check(_ raw: Any?) -> JSONValue? {
        guard let object = raw as? [String: Any], let kind = object["kind"] as? String, kinds.contains(kind),
              let data = try? JSONSerialization.data(withJSONObject: object), data.count <= maxBytes,
              let value = JSONValue(any: object) else { return nil }
        return finite(value) ? value : nil
    }

    private static func finite(_ value: JSONValue) -> Bool {
        switch value {
        case .number(let n): return n.isFinite
        case .array(let a): return a.count <= 64 && a.allSatisfy(finite)
        case .object(let o): return o.values.allSatisfy(finite)
        default: return true
        }
    }
}
