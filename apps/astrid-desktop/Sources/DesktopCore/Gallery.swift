import Foundation

/// One saved drawing, for the Diagrams page: the reply that drew it and the
/// chat it belongs to.
public struct DiagramItem: Equatable, Identifiable, Sendable {
    /// The reply's message id; a reply draws at most one diagram.
    public var id: UUID
    public var chatID: UUID
    public var chatTitle: String
    /// The drawing's own title shape, else the chat's title.
    public var title: String
    public var shapes: [JSONValue]
    public var at: Date
}

public enum ChatGallery {
    /// Every drawing in `chats`, newest first; ties by id so the order is stable.
    public static func diagrams(in chats: [ChatSession]) -> [DiagramItem] {
        chats.flatMap { chat in
            chat.messages.compactMap { message -> DiagramItem? in
                guard message.role == .astrid, let shapes = message.diagram, !shapes.isEmpty else { return nil }
                return DiagramItem(id: message.id, chatID: chat.id, chatTitle: chat.title,
                                   title: message.diagramTitle ?? chat.title, shapes: shapes, at: message.at)
            }
        }
        .sorted { $0.at != $1.at ? $0.at > $1.at : $0.id.uuidString < $1.id.uuidString }
    }
}

/// A minimal SVG path reader for Astrid's mark: absolute M, L, H, V, C and Z,
/// which is all the logo's paths use. Anything else ends the parse.
public enum SVGPath {
    public enum Command: Equatable, Sendable {
        case move(Double, Double)
        case line(Double, Double)
        case cubic(Double, Double, Double, Double, Double, Double)
        case close
    }

    public static func parse(_ d: String) -> [Command] {
        var tokens: [String] = []
        var number = ""
        func flush() {
            if !number.isEmpty { tokens.append(number); number = "" }
        }
        for ch in d {
            if ch.isLetter && ch != "e" && ch != "E" {
                flush()
                tokens.append(String(ch))
            } else if ch == "," || ch.isWhitespace {
                flush()
            } else if ch == "-" && !number.isEmpty && !number.hasSuffix("e") && !number.hasSuffix("E") {
                flush()
                number = "-"
            } else {
                number.append(ch)
            }
        }
        flush()

        var out: [Command] = []
        var i = 0
        var x = 0.0, y = 0.0
        var command = ""
        func next() -> Double? {
            guard i < tokens.count, let v = Double(tokens[i]) else { return nil }
            i += 1
            return v
        }
        while i < tokens.count {
            if Double(tokens[i]) == nil {
                command = tokens[i]
                i += 1
            }
            switch command {
            case "M", "L":
                guard let a = next(), let b = next() else { return out }
                x = a; y = b
                out.append(command == "M" ? .move(a, b) : .line(a, b))
                // Pairs after a move are lines.
                if command == "M" { command = "L" }
            case "H":
                guard let a = next() else { return out }
                x = a
                out.append(.line(x, y))
            case "V":
                guard let b = next() else { return out }
                y = b
                out.append(.line(x, y))
            case "C":
                guard let a = next(), let b = next(), let c = next(), let e = next(), let f = next(), let g = next() else { return out }
                x = f; y = g
                out.append(.cubic(a, b, c, e, f, g))
            case "Z", "z":
                out.append(.close)
            default:
                return out
            }
        }
        return out
    }
}
