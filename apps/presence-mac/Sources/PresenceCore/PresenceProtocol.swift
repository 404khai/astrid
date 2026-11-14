import CoreGraphics
import Foundation

/// Presence protocol v1 (ADR 0026): runtime → host events, one JSON object per
/// line. Coordinates are global display points with the primary display's
/// top-left corner as origin and y growing downward (Quartz convention). With
/// `display`, coordinates are relative to that display's top-left corner.
public enum PresenceProtocol {
    public static let version = 1

    /// Bounds that keep a malformed or hostile stream finite.
    public enum Limits {
        public static let maxLineBytes = 16 * 1024
        public static let maxScriptBytes = 1024 * 1024
        public static let maxTextCharacters = 1_000
        public static let maxShapesPerSketch = 32
        public static let maxInkShapes = 256
        public static let maxCoordinate = 100_000.0
        public static let maxRadius = 10_000.0
        public static let maxMilliseconds = 60_000
        public static let maxDisplayIndex = 15
    }
}

/// A point in protocol space: global points, or display-relative points.
public struct PresencePoint: Equatable, Sendable {
    public var x: Double
    public var y: Double
    public var display: Int?

    public init(x: Double, y: Double, display: Int? = nil) {
        self.x = x
        self.y = y
        self.display = display
    }
}

public enum InkShape: Equatable, Sendable {
    case circle(center: CGPoint, radius: Double)
    case arrow(from: CGPoint, to: CGPoint)
    case line(from: CGPoint, to: CGPoint)
}

public enum PresenceEvent: Equatable, Sendable {
    /// Fade in; optionally reposition first.
    case show(at: PresencePoint?)
    case hide
    /// Glide Astrid's cursor tip (the hotspot) to a point. `cursor` picks the
    /// style that says what to do there; nil is Astrid's own arrow.
    case pointAt(PresencePoint, cursor: CursorStyle? = nil)
    /// Subtitle text. Empty text clears. `final == false` is a streaming
    /// partial that never auto-clears; `durationMs` overrides the reading time.
    case say(text: String, final: Bool, durationMs: Int?)
    /// Ink shapes, all in the same coordinate space (`display` as for points).
    case sketch(shapes: [InkShape], display: Int?)
    case clearInk
    /// Script pacing: hold later events for a number of milliseconds.
    case wait(ms: Int)
    case quit
    /// Play the Distortion Waves signal now (an operator session went live),
    /// unless Reduce Motion or --no-intro; ignored while a wave is running.
    case wave
    /// Capture one display at 1x point resolution, excluding Astrid's own
    /// overlay (ADR 0027). Answered on stdout with `frame` or `capture_failed`.
    case capture(requestId: String, display: Int)
    /// ADR 0029 (resident host only): timers and alarms.
    case timerStart(id: String, seconds: Int, label: String)
    case timerCancel(id: String)
    case alarmSet(id: String, at: Date, label: String)
    case timersList
}

public struct PresenceProtocolError: Error, Equatable, CustomStringConvertible {
    public var message: String
    public init(_ message: String) { self.message = message }
    public var description: String { message }
}

// MARK: - Parsing

extension PresenceProtocol {
    /// Parses one JSON line. Unknown fields are ignored so v1 can grow
    /// additively; unknown types and wrong versions are rejected.
    public static func parse(line: String) throws -> PresenceEvent {
        guard line.utf8.count <= Limits.maxLineBytes else {
            throw PresenceProtocolError("line exceeds \(Limits.maxLineBytes) bytes")
        }
        let object: Any
        do {
            object = try JSONSerialization.jsonObject(with: Data(line.utf8))
        } catch {
            throw PresenceProtocolError("invalid JSON")
        }
        guard let fields = object as? [String: Any] else {
            throw PresenceProtocolError("expected a JSON object")
        }
        let reader = FieldReader(fields)
        guard let v = try reader.int("v") else {
            throw PresenceProtocolError("missing \"v\"")
        }
        guard v == version else {
            throw PresenceProtocolError("unsupported protocol version \(v)")
        }
        guard let type = try reader.string("type") else {
            throw PresenceProtocolError("missing \"type\"")
        }
        switch type {
        case "show":
            let hasPoint = fields["x"] != nil || fields["y"] != nil
            return .show(at: hasPoint ? try reader.point() : nil)
        case "hide":
            return .hide
        case "point_at":
            var cursor: CursorStyle?
            if let name = try reader.string("cursor") {
                guard let style = CursorStyle(rawValue: name) else {
                    throw PresenceProtocolError("unknown cursor \"\(name.prefix(32))\"")
                }
                cursor = style
            }
            return .pointAt(try reader.point(), cursor: cursor)
        case "say":
            let text = try reader.string("text") ?? ""
            guard text.count <= Limits.maxTextCharacters else {
                throw PresenceProtocolError("text exceeds \(Limits.maxTextCharacters) characters")
            }
            let duration = try reader.int("duration_ms")
            if let duration { try checkMilliseconds(duration, "duration_ms") }
            return .say(text: text, final: try reader.bool("final") ?? true, durationMs: duration)
        case "sketch":
            guard let raw = fields["shapes"] as? [Any] else {
                throw PresenceProtocolError("\"shapes\" must be an array")
            }
            guard !raw.isEmpty, raw.count <= Limits.maxShapesPerSketch else {
                throw PresenceProtocolError("\"shapes\" must hold 1...\(Limits.maxShapesPerSketch) shapes")
            }
            let shapes = try raw.enumerated().map { index, value -> InkShape in
                guard let shape = value as? [String: Any] else {
                    throw PresenceProtocolError("shape \(index) must be an object")
                }
                return try parseShape(FieldReader(shape), index: index)
            }
            return .sketch(shapes: shapes, display: try reader.display())
        case "clear_ink":
            return .clearInk
        case "wait":
            guard let ms = try reader.int("ms") else { throw PresenceProtocolError("missing \"ms\"") }
            try checkMilliseconds(ms, "ms")
            return .wait(ms: ms)
        case "quit":
            return .quit
        case "wave":
            return .wave
        case "capture":
            guard let id = try reader.string("request_id"), isRequestId(id) else {
                throw PresenceProtocolError("\"request_id\" must be 1...64 characters of [A-Za-z0-9-]")
            }
            guard let display = try reader.display() else { throw PresenceProtocolError("missing \"display\"") }
            return .capture(requestId: id, display: display)
        case "timer_start":
            let id = try requestId(reader)
            guard let seconds = try reader.int("seconds") else { throw PresenceProtocolError("missing \"seconds\"") }
            return .timerStart(id: id, seconds: seconds, label: try reader.string("label") ?? "")
        case "timer_cancel":
            return .timerCancel(id: try requestId(reader))
        case "alarm_set":
            let id = try requestId(reader)
            guard let text = try reader.string("at"), let at = isoDate(text) else {
                throw PresenceProtocolError("\"at\" must be an ISO-8601 time with a zone, e.g. 2026-11-12T07:30:00Z")
            }
            return .alarmSet(id: id, at: at, label: try reader.string("label") ?? "")
        case "timers_list":
            return .timersList
        default:
            throw PresenceProtocolError("unknown type \"\(type)\"")
        }
    }

    private static func parseShape(_ reader: FieldReader, index: Int) throws -> InkShape {
        switch try reader.string("kind") ?? "" {
        case "circle":
            guard let r = try reader.number("r"), r > 0, r <= Limits.maxRadius else {
                throw PresenceProtocolError("shape \(index): \"r\" must be in (0, \(Int(Limits.maxRadius))]")
            }
            return .circle(center: try reader.xy(), radius: r)
        case "arrow":
            return .arrow(from: try reader.pair("from"), to: try reader.pair("to"))
        case "line":
            return .line(from: try reader.pair("from"), to: try reader.pair("to"))
        case let kind:
            throw PresenceProtocolError("shape \(index): unknown kind \"\(kind)\"")
        }
    }

    public static func isRequestId(_ id: String) -> Bool {
        (1...64).contains(id.count) && id.allSatisfy { $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "-") }
    }

    private static func requestId(_ reader: FieldReader) throws -> String {
        guard let id = try reader.string("id"), isRequestId(id) else {
            throw PresenceProtocolError("\"id\" must be 1...64 characters of [A-Za-z0-9-]")
        }
        return id
    }

    public static func isoDate(_ text: String) -> Date? {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        if let date = formatter.date(from: text) { return date }
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter.date(from: text)
    }

    private static func checkMilliseconds(_ value: Int, _ name: String) throws {
        guard (0...Limits.maxMilliseconds).contains(value) else {
            throw PresenceProtocolError("\"\(name)\" must be in 0...\(Limits.maxMilliseconds)")
        }
    }

    /// A parsed script: valid events with their 1-based line numbers, plus
    /// every rejected line. Blank lines and lines starting with `#` are skipped.
    public struct Script: Equatable {
        public var events: [(line: Int, event: PresenceEvent)]
        public var errors: [(line: Int, error: PresenceProtocolError)]

        public static func == (a: Script, b: Script) -> Bool {
            a.events.map(\.line) == b.events.map(\.line) && a.events.map(\.event) == b.events.map(\.event)
                && a.errors.map(\.line) == b.errors.map(\.line) && a.errors.map(\.error) == b.errors.map(\.error)
        }
    }

    public static func parseScript(_ text: String) -> Script {
        var script = Script(events: [], errors: [])
        for (offset, raw) in text.split(separator: "\n", omittingEmptySubsequences: false).enumerated() {
            let line = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            if line.isEmpty || line.hasPrefix("#") { continue }
            do {
                script.events.append((offset + 1, try parse(line: line)))
            } catch let error as PresenceProtocolError {
                script.errors.append((offset + 1, error))
            } catch {
                script.errors.append((offset + 1, PresenceProtocolError("\(error)")))
            }
        }
        return script
    }
}

/// Typed access to a decoded JSON object with explicit errors.
private struct FieldReader {
    let fields: [String: Any]
    init(_ fields: [String: Any]) { self.fields = fields }

    func number(_ key: String) throws -> Double? {
        guard let value = fields[key], !(value is NSNull) else { return nil }
        guard let number = value as? NSNumber, !isBool(number) else {
            throw PresenceProtocolError("\"\(key)\" must be a number")
        }
        let double = number.doubleValue
        guard double.isFinite, abs(double) <= PresenceProtocol.Limits.maxCoordinate else {
            throw PresenceProtocolError("\"\(key)\" is out of range")
        }
        return double
    }

    func int(_ key: String) throws -> Int? {
        guard let double = try number(key) else { return nil }
        guard double == double.rounded() else { throw PresenceProtocolError("\"\(key)\" must be an integer") }
        return Int(double)
    }

    func string(_ key: String) throws -> String? {
        guard let value = fields[key], !(value is NSNull) else { return nil }
        guard let string = value as? String else { throw PresenceProtocolError("\"\(key)\" must be a string") }
        return string
    }

    func bool(_ key: String) throws -> Bool? {
        guard let value = fields[key], !(value is NSNull) else { return nil }
        guard let number = value as? NSNumber, isBool(number) else {
            throw PresenceProtocolError("\"\(key)\" must be a boolean")
        }
        return number.boolValue
    }

    func display() throws -> Int? {
        guard let display = try int("display") else { return nil }
        guard (0...PresenceProtocol.Limits.maxDisplayIndex).contains(display) else {
            throw PresenceProtocolError("\"display\" must be in 0...\(PresenceProtocol.Limits.maxDisplayIndex)")
        }
        return display
    }

    func xy() throws -> CGPoint {
        guard let x = try number("x"), let y = try number("y") else {
            throw PresenceProtocolError("\"x\" and \"y\" are required")
        }
        return CGPoint(x: x, y: y)
    }

    func point() throws -> PresencePoint {
        let p = try xy()
        return PresencePoint(x: p.x, y: p.y, display: try display())
    }

    func pair(_ key: String) throws -> CGPoint {
        guard let values = fields[key] as? [Any], values.count == 2 else {
            throw PresenceProtocolError("\"\(key)\" must be [x, y]")
        }
        let reader = FieldReader(["x": values[0], "y": values[1]])
        do { return try reader.xy() } catch { throw PresenceProtocolError("\"\(key)\" must be [x, y] numbers in range") }
    }

    private func isBool(_ number: NSNumber) -> Bool {
        CFGetTypeID(number) == CFBooleanGetTypeID()
    }
}

// MARK: - Encoding

extension PresenceProtocol {
    /// Canonical single-line JSON (sorted keys) for an event; `parse` inverts it.
    public static func encode(_ event: PresenceEvent) -> String {
        var fields: [String: Any] = ["v": version]
        func put(_ point: PresencePoint) {
            fields["x"] = point.x
            fields["y"] = point.y
            if let display = point.display { fields["display"] = display }
        }
        switch event {
        case let .show(at):
            fields["type"] = "show"
            if let at { put(at) }
        case .hide:
            fields["type"] = "hide"
        case let .pointAt(point, cursor):
            fields["type"] = "point_at"
            put(point)
            if let cursor { fields["cursor"] = cursor.rawValue }
        case let .say(text, final, durationMs):
            fields["type"] = "say"
            fields["text"] = text
            if !final { fields["final"] = false }
            if let durationMs { fields["duration_ms"] = durationMs }
        case let .sketch(shapes, display):
            fields["type"] = "sketch"
            if let display { fields["display"] = display }
            fields["shapes"] = shapes.map { shape -> [String: Any] in
                switch shape {
                case let .circle(center, radius):
                    return ["kind": "circle", "x": center.x, "y": center.y, "r": radius]
                case let .arrow(from, to):
                    return ["kind": "arrow", "from": [from.x, from.y], "to": [to.x, to.y]]
                case let .line(from, to):
                    return ["kind": "line", "from": [from.x, from.y], "to": [to.x, to.y]]
                }
            }
        case .clearInk:
            fields["type"] = "clear_ink"
        case let .wait(ms):
            fields["type"] = "wait"
            fields["ms"] = ms
        case .quit:
            fields["type"] = "quit"
        case .wave:
            fields["type"] = "wave"
        case let .capture(requestId, display):
            fields["type"] = "capture"
            fields["request_id"] = requestId
            fields["display"] = display
        case let .timerStart(id, seconds, label):
            fields["type"] = "timer_start"
            fields["id"] = id
            fields["seconds"] = seconds
            fields["label"] = label
        case let .timerCancel(id):
            fields["type"] = "timer_cancel"
            fields["id"] = id
        case let .alarmSet(id, at, label):
            fields["type"] = "alarm_set"
            fields["id"] = id
            fields["at"] = ISO8601DateFormatter().string(from: at)
            fields["label"] = label
        case .timersList:
            fields["type"] = "timers_list"
        }
        let data = try! JSONSerialization.data(withJSONObject: fields, options: [.sortedKeys, .withoutEscapingSlashes])
        return String(decoding: data, as: UTF8.self)
    }
}
