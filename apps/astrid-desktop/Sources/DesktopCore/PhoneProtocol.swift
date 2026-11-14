import Foundation

/// What a paired phone may ask of the Mac over the link (ADR 0039). Nothing
/// here reaches a tool: asks run like the desktop's own, read-only, with no
/// mail source.
public enum PhoneRequest: Equatable {
    /// Send the chat list.
    case sync
    /// Send this chat, and keep sending it as it changes.
    case open(UUID)
    /// Ask Astrid, in this chat or a new one (nil).
    case ask(chat: UUID?, text: String)
    /// Stop the running ask.
    case cancel
    case ping
    /// Rename a chat or change its icon or colour (`ChatStyle`).
    case style(chat: UUID, title: String?, icon: String?, color: String?)
    /// Send the newest drawings.
    case diagrams

    public static func parse(_ message: [String: Any]) -> PhoneRequest? {
        switch message["type"] as? String {
        case "sync": return .sync
        case "open":
            guard let id = (message["chat"] as? String).flatMap(UUID.init(uuidString:)) else { return nil }
            return .open(id)
        case "ask":
            guard let raw = message["text"] as? String else { return nil }
            let text = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return nil }
            let chat = (message["chat"] as? String).flatMap(UUID.init(uuidString:))
            if message["chat"] is String, chat == nil { return nil }
            return .ask(chat: chat, text: String(text.prefix(LinkLimits.maxAsk)))
        case "cancel": return .cancel
        case "ping": return .ping
        case "style":
            guard let id = (message["chat"] as? String).flatMap(UUID.init(uuidString:)) else { return nil }
            let title = message["title"] as? String, icon = message["icon"] as? String, color = message["color"] as? String
            guard title != nil || icon != nil || color != nil else { return nil }
            return .style(chat: id, title: title.map { String($0.prefix(ChatStyle.maxTitle * 2)) }, icon: icon, color: color)
        case "diagrams": return .diagrams
        default: return nil
        }
    }
}

/// What the Mac sends a phone.
public enum PhoneMessage {
    private static let dates: ISO8601DateFormatter = ISO8601DateFormatter()

    /// The chat list: most recent first, with a line of each chat's last message.
    public static func chats(_ chats: [ChatSession], busy: UUID?) -> [String: Any] {
        let rows: [[String: Any]] = chats.prefix(LinkLimits.maxListed).map { chat in
            let last = chat.messages.last
            let preview = last.map { message -> String in
                let text = message.text.split(whereSeparator: \.isNewline).first.map(String.init) ?? ""
                return String((text.isEmpty ? (message.diagramTitle.map { "Drew: \($0)" } ?? "") : text).prefix(120))
            } ?? ""
            return [
                "id": chat.id.uuidString,
                "title": chat.title,
                "updated": dates.string(from: chat.updated),
                "preview": preview,
                "drawings": chat.messages.filter { $0.diagram != nil }.count,
                "icon": chat.icon ?? "",
                "color": chat.color ?? "",
            ]
        }
        var message: [String: Any] = ["type": "chats", "chats": rows]
        if let busy { message["busy"] = busy.uuidString }
        return message
    }

    /// One whole chat, as the desktop saves it.
    public static func chat(_ chat: ChatSession) -> [String: Any]? {
        guard let data = try? ChatStore.encode(chat),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
        return ["type": "chat", "chat": object]
    }

    /// An event of the running ask, desktop's or phone's, so either can watch.
    public static func live(chat: UUID, reply: UUID, event: AskEvent) -> [String: Any] {
        ["type": "live", "chat": chat.uuidString, "reply": reply.uuidString, "event": event.json]
    }

    /// An ask started on the phone, and which chat it is in.
    public static func started(chat: UUID, reply: UUID) -> [String: Any] {
        ["type": "started", "chat": chat.uuidString, "reply": reply.uuidString]
    }

    /// The newest drawings, shapes included, up to `maxDiagrams`.
    public static func diagrams(_ items: [DiagramItem]) -> [String: Any] {
        let rows: [[String: Any]] = items.prefix(LinkLimits.maxDiagrams).map { item in
            ["id": item.id.uuidString, "chat": item.chatID.uuidString, "title": item.title,
             "at": dates.string(from: item.at), "shapes": item.shapes.map(\.any)]
        }
        return ["type": "diagrams", "items": rows]
    }

    public static func refused(_ reason: String) -> [String: Any] {
        ["type": "refused", "message": reason]
    }
}

extension AskEvent {
    /// The event as the runtime's line shape, for the phone.
    public var json: [String: Any] {
        switch self {
        case .ready: return ["type": "ready"]
        case .state(let state, let label): return ["type": "state", "state": state, "label": label]
        case .delta(let text): return ["type": "delta", "text": text]
        // Approvals are answered on the Mac; phone asks have no mail, so none come.
        case .approval(_, let title): return ["type": "approval", "title": title]
        case .answer(let text, let brief, let drew):
            var event: [String: Any] = ["type": "answer", "text": text, "canvas": drew]
            if let brief { event["brief"] = brief.any }
            return event
        case .canvasBegin: return ["type": "canvas", "event": "begin"]
        case .canvasShape(let shape): return ["type": "canvas", "event": "shape", "shape": shape.any]
        case .canvasEnd: return ["type": "canvas", "event": "end"]
        case .canvasCancel: return ["type": "canvas", "event": "cancel"]
        case .error(let message): return ["type": "error", "message": message]
        case .done(let outcome): return ["type": "done", "outcome": outcome]
        }
    }
}
