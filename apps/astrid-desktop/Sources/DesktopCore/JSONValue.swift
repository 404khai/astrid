import Foundation

/// A JSON value kept as-is: canvas shapes and briefs are validated by the
/// runtime (ADR 0036), and the desktop store saves them without reinterpreting
/// their fields, so the canvas page gets back exactly what the runtime sent.
public enum JSONValue: Codable, Equatable, Sendable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    /// From a `JSONSerialization` object; nil for anything that isn't JSON.
    public init?(any: Any) {
        switch any {
        case is NSNull: self = .null
        case let number as NSNumber:
            // JSONSerialization gives booleans as NSNumber too.
            if CFGetTypeID(number) == CFBooleanGetTypeID() { self = .bool(number.boolValue) } else { self = .number(number.doubleValue) }
        case let string as String: self = .string(string)
        case let list as [Any]:
            var values: [JSONValue] = []
            for item in list {
                guard let value = JSONValue(any: item) else { return nil }
                values.append(value)
            }
            self = .array(values)
        case let map as [String: Any]:
            var values: [String: JSONValue] = [:]
            for (key, item) in map {
                guard let value = JSONValue(any: item) else { return nil }
                values[key] = value
            }
            self = .object(values)
        default: return nil
        }
    }

    /// Back to a `JSONSerialization` object.
    public var any: Any {
        switch self {
        case .null: return NSNull()
        case .bool(let value): return value
        case .number(let value): return value
        case .string(let value): return value
        case .array(let values): return values.map(\.any)
        case .object(let values): return values.mapValues(\.any)
        }
    }

    public subscript(key: String) -> JSONValue? {
        if case .object(let values) = self { return values[key] }
        return nil
    }

    public var string: String? {
        if case .string(let value) = self { return value }
        return nil
    }

    public var array: [JSONValue]? {
        if case .array(let values) = self { return values }
        return nil
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() { self = .null }
        else if let value = try? container.decode(Bool.self) { self = .bool(value) }
        else if let value = try? container.decode(Double.self) { self = .number(value) }
        else if let value = try? container.decode(String.self) { self = .string(value) }
        else if let value = try? container.decode([JSONValue].self) { self = .array(value) }
        else { self = .object(try container.decode([String: JSONValue].self)) }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .null: try container.encodeNil()
        case .bool(let value): try container.encode(value)
        case .number(let value): try container.encode(value)
        case .string(let value): try container.encode(value)
        case .array(let values): try container.encode(values)
        case .object(let values): try container.encode(values)
        }
    }
}
