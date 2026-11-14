import CoreGraphics
import Foundation

/// ADR 0038 "Do it": the one bounded action an operator step may perform on
/// this Mac. The runtime validates it first; the host validates again here
/// before any input is posted.
public enum OperatorAction: Equatable, Sendable {
    case click
    case type(String)
    case key(String)
    case scroll(down: Bool, notches: Int)

    public static let maxTypeCharacters = 200
    /// macOS virtual key codes for the keys "Do it" may press (no modifiers).
    public static let keyCodes: [String: UInt16] = [
        "enter": 36, "tab": 48, "escape": 53, "space": 49,
        "left": 123, "right": 124, "down": 125, "up": 126,
    ]

    public static func parse(_ fields: [String: Any]) throws -> OperatorAction {
        switch fields["kind"] as? String {
        case "click":
            return .click
        case "type":
            guard let text = fields["text"] as? String, (1...maxTypeCharacters).contains(text.count),
                  !text.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }) else {
                throw PresenceProtocolError("type text must be 1...\(maxTypeCharacters) characters, one line")
            }
            return .type(text)
        case "key":
            guard let key = fields["key"] as? String, keyCodes[key] != nil else {
                throw PresenceProtocolError("key must be one of \(keyCodes.keys.sorted().joined(separator: ", "))")
            }
            return .key(key)
        case "scroll":
            guard let direction = fields["direction"] as? String, ["up", "down"].contains(direction),
                  let notches = (fields["notches"] as? NSNumber)?.intValue, (1...10).contains(notches) else {
                throw PresenceProtocolError("scroll needs direction up/down and 1...10 notches")
            }
            return .scroll(down: direction == "down", notches: notches)
        default:
            throw PresenceProtocolError("action kind must be click, type, key or scroll")
        }
    }
}

/// A coarse grayscale copy of a look frame, kept in the host's memory only so
/// "Do it" can refuse to act if the screen around the target has changed since
/// the look the step was planned on (the ADR 0023 freshness rule, adapted to a
/// human reading a card first). It never leaves the process and is replaced
/// by the next look.
public struct Fingerprint: Equatable, Sendable {
    /// Display points per fingerprint cell.
    public static let cell = 4
    /// Half-size, in points, of the square around the target that must match.
    public static let radius = 48
    /// Largest mean absolute difference (0...255) still counted as unchanged.
    public static let tolerance = 6.0

    public let width: Int
    public let height: Int
    public let cells: [UInt8]

    public init(width: Int, height: Int, cells: [UInt8]) {
        self.width = width
        self.height = height
        self.cells = cells
    }

    /// Averages 8-bit RGBA pixels (row-major, `bytesPerRow`) into cells.
    public init(rgba: [UInt8], width: Int, height: Int, bytesPerRow: Int) {
        let cell = Self.cell
        let w = max(1, width / cell), h = max(1, height / cell)
        var cells = [UInt8](repeating: 0, count: w * h)
        for cy in 0..<h {
            for cx in 0..<w {
                var sum = 0, count = 0
                for y in (cy * cell)..<min(height, cy * cell + cell) {
                    for x in (cx * cell)..<min(width, cx * cell + cell) {
                        let i = y * bytesPerRow + x * 4
                        guard i + 2 < rgba.count else { continue }
                        sum += (Int(rgba[i]) * 299 + Int(rgba[i + 1]) * 587 + Int(rgba[i + 2]) * 114) / 1000
                        count += 1
                    }
                }
                cells[cy * w + cx] = UInt8(count == 0 ? 0 : sum / count)
            }
        }
        self.init(width: w, height: h, cells: cells)
    }

    /// Whether the area around `point` (display points) is the same in both.
    public func matches(_ other: Fingerprint, around point: CGPoint) -> Bool {
        guard width == other.width, height == other.height else { return false }
        let cell = Double(Self.cell), r = Double(Self.radius)
        let x0 = max(0, Int((point.x - r) / cell)), x1 = min(width - 1, Int((point.x + r) / cell))
        let y0 = max(0, Int((point.y - r) / cell)), y1 = min(height - 1, Int((point.y + r) / cell))
        guard x0 <= x1, y0 <= y1 else { return false }
        var total = 0, count = 0
        for y in y0...y1 {
            for x in x0...x1 {
                total += abs(Int(cells[y * width + x]) - Int(other.cells[y * width + x]))
                count += 1
            }
        }
        return Double(total) / Double(count) <= Self.tolerance
    }
}
