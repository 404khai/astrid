import Foundation

/// One message in a desktop chat (ADR 0037): what the user sent, or Astrid's
/// reply with its brief and the drawing it made, if any.
public struct ChatMessage: Codable, Equatable, Identifiable, Sendable {
    public enum Role: String, Codable, Sendable { case user, astrid }

    public var id: UUID
    public var role: Role
    public var text: String
    /// The reply's brief (ADR 0033), as the runtime sent it.
    public var brief: JSONValue?
    /// The validated canvas shapes of the reply's drawing, in order (ADR 0036).
    public var diagram: [JSONValue]?
    /// How Astrid's run ended: completed, cancelled, failed or limit.
    public var outcome: String?
    public var at: Date

    public init(id: UUID = UUID(), role: Role, text: String, brief: JSONValue? = nil, diagram: [JSONValue]? = nil, outcome: String? = nil, at: Date = Date()) {
        self.id = id
        self.role = role
        self.text = String(text.prefix(ChatLimits.maxText))
        self.brief = brief
        self.diagram = diagram.map { Array($0.prefix(ChatLimits.maxShapes)) }
        self.outcome = outcome
        self.at = at
    }

    /// The drawing's title shape, if it has one.
    public var diagramTitle: String? {
        diagram?.first { $0["shape"]?.string == "title" }?["text"]?.string
    }
}

/// A chat: the user's messages and Astrid's replies, named from the first message.
public struct ChatSession: Codable, Equatable, Identifiable, Sendable {
    public static let currentVersion = 1

    public var v: Int
    public var id: UUID
    public var title: String
    public var created: Date
    public var updated: Date
    public var messages: [ChatMessage]
    /// The chat's icon and colour, chosen by the user (`ChatStyle`); nil for
    /// the defaults. Older files have neither.
    public var icon: String?
    public var color: String?

    /// A chat starts with the first message the user sends; it is named from it.
    public init(id: UUID = UUID(), first: ChatMessage) {
        v = Self.currentVersion
        self.id = id
        title = ChatTitle.derive(from: first.text)
        created = first.at
        updated = first.at
        messages = [first]
    }

    public mutating func append(_ message: ChatMessage) {
        messages.append(message)
        updated = max(updated, message.at)
    }

    /// Earlier turns for a follow-up, in the runtime's `history` shape
    /// (`src/presence_ask.rs`). A drawing is described by its title.
    public func history() -> [[String: String]] {
        messages.compactMap { message in
            var text = message.text.trimmingCharacters(in: .whitespacesAndNewlines)
            if message.role == .astrid {
                if text.isEmpty, let brief = message.brief {
                    text = [brief["title"]?.string, brief["summary"]?.string].compactMap { $0 }.joined(separator: ". ")
                }
                if let title = message.diagramTitle { text += (text.isEmpty ? "" : " ") + "[Drew on the canvas: \(title)]" }
            }
            guard !text.isEmpty else { return nil }
            return ["role": message.role.rawValue, "text": text]
        }
    }
}

/// The icons and colours a chat can wear, as names both apps map to their own
/// glyphs and the same hex colours.
public enum ChatStyle {
    public static let icons = ["chat", "star", "brain", "code", "book", "heart", "bolt", "globe", "flask", "music", "house", "camera", "map", "leaf"]
    public static let colors: [(name: String, hex: String)] = [
        ("blue", "#4459F9"), ("violet", "#8A5CFF"), ("pink", "#E5487F"), ("red", "#E5484D"), ("orange", "#F07A2B"),
        ("yellow", "#E8B400"), ("green", "#3DBE6A"), ("teal", "#00C2A8"), ("grey", "#8E8E99"),
    ]
    public static let maxTitle = 80

    public static func hex(_ name: String?) -> String? { colors.first { $0.name == name }?.hex }

    /// Applies a rename, icon or colour; returns nil if nothing valid changed.
    /// An empty icon or colour resets it to the default.
    public static func apply(to chat: ChatSession, title: String?, icon: String?, color: String?) -> ChatSession? {
        var next = chat
        if let title {
            let clean = title.trimmingCharacters(in: .whitespacesAndNewlines).replacingOccurrences(of: "\n", with: " ")
            guard !clean.isEmpty else { return nil }
            next.title = String(clean.prefix(maxTitle))
        }
        if let icon {
            guard icon.isEmpty || icons.contains(icon) else { return nil }
            next.icon = icon.isEmpty ? nil : icon
        }
        if let color {
            guard color.isEmpty || hex(color) != nil else { return nil }
            next.color = color.isEmpty ? nil : color
        }
        return next == chat ? nil : next
    }
}

public enum ChatLimits {
    /// Characters kept per message.
    public static let maxText = 64 * 1024
    /// Shapes kept per drawing; the runtime sends at most 200.
    public static let maxShapes = 200
    /// Chat files larger than this are not loaded.
    public static let maxFileBytes = 8 * 1024 * 1024
    /// Chats listed in the sidebar, most recent first.
    public static let maxChats = 500
}

/// Chat titles come from the first message, deterministically: its first
/// sentence, without greetings and polite openers, cut to a few words.
public enum ChatTitle {
    public static let fallback = "New chat"
    static let maxWords = 6
    static let maxCharacters = 40

    /// Openers dropped from the front, longest first, case-insensitive.
    static let openers = [
        "i would like you to", "i'd like you to", "i want you to", "i need you to",
        "can you please", "could you please", "would you please", "will you please",
        "can you", "could you", "would you", "will you",
        "hey astrid", "hi astrid", "hello astrid", "ok astrid", "okay astrid",
        "astrid", "hey", "hi", "hello", "please", "so", "ok", "okay",
    ]

    public static func derive(from text: String) -> String {
        var words = text.split(whereSeparator: \.isWhitespace).joined(separator: " ")
        // First sentence (or line): stop at the first terminator that ends a word.
        if let end = words.firstIndex(where: { ".?!;".contains($0) }), words.distance(from: words.startIndex, to: end) >= 3 {
            words = String(words[..<end])
        }
        var rest = Substring(words)
        var stripped = true
        while stripped {
            stripped = false
            rest = rest.drop { $0.isWhitespace || ",:;-—–.!?".contains($0) }
            for opener in openers where rest.lowercased().hasPrefix(opener) {
                let after = rest.dropFirst(opener.count)
                // Only whole words: "hiking" must not lose "hi".
                if let next = after.first, next.isLetter || next.isNumber { continue }
                rest = after
                stripped = true
                break
            }
        }
        var title = rest.trimmingCharacters(in: .whitespaces)
        title = title.trimmingCharacters(in: CharacterSet(charactersIn: ",:;-—–.!?\"'“”‘’ "))
        guard !title.isEmpty else { return fallback }
        let list = title.split(separator: " ")
        var kept: [Substring] = []
        for word in list {
            let length = kept.reduce(0) { $0 + $1.count + 1 } + word.count
            if kept.count == maxWords || (length > maxCharacters && !kept.isEmpty) { break }
            kept.append(word)
        }
        var result = kept.joined(separator: " ")
        if result.count > maxCharacters { result = String(result.prefix(maxCharacters - 1)) }
        if kept.count < list.count || result.count < kept.joined(separator: " ").count {
            result = result.trimmingCharacters(in: CharacterSet(charactersIn: ",:;-—– ")) + "…"
        }
        return result.prefix(1).uppercased() + result.dropFirst()
    }
}
