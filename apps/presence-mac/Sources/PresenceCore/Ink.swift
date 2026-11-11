import CoreGraphics
import Foundation

/// Hand-drawn ink geometry. Deterministic: the same shape always yields the
/// same path, so redraws and tests are stable.
public enum InkGeometry {
    /// Resolves a sketch's shapes to global presence space.
    public static func resolve(_ shapes: [InkShape], display: Int?, in layout: DisplayLayout) throws -> [InkShape] {
        try shapes.map { shape in
            switch shape {
            case let .circle(center, radius):
                return .circle(center: try layout.resolve(center, display: display), radius: radius)
            case let .arrow(from, to):
                return .arrow(from: try layout.resolve(from, display: display), to: try layout.resolve(to, display: display))
            case let .line(from, to):
                return .line(from: try layout.resolve(from, display: display), to: try layout.resolve(to, display: display))
            }
        }
    }

    public static func path(for shape: InkShape) -> CGPath {
        switch shape {
        case let .circle(center, radius): return circle(center: center, radius: radius)
        case let .arrow(from, to): return arrow(from: from, to: to)
        case let .line(from, to): return stroke(from: from, to: to)
        }
    }

    /// A slightly wobbly loop that starts above-left and overshoots its start,
    /// like a circle drawn by hand around a target.
    public static func circle(center: CGPoint, radius: Double, samples: Int = 72) -> CGPath {
        let path = CGMutablePath()
        let seed = Self.seed(center.x, center.y, radius)
        let start = -0.6 * Double.pi
        let sweep = 2.15 * Double.pi
        for i in 0...samples {
            let t = Double(i) / Double(samples)
            let angle = start + sweep * t
            let wobble = 1 + 0.035 * sin(3 * angle + seed) + 0.07 * t
            let p = CGPoint(
                x: center.x + cos(angle) * radius * 1.08 * wobble,
                y: center.y + sin(angle) * radius * 0.92 * wobble)
            if i == 0 { path.move(to: p) } else { path.addLine(to: p) }
        }
        return path
    }

    /// A slightly bowed shaft, then two head strokes at the tip.
    public static func arrow(from: CGPoint, to: CGPoint) -> CGPath {
        let path = CGMutablePath(); path.addPath(stroke(from: from, to: to))
        let dx = to.x - from.x, dy = to.y - from.y
        let length = (dx * dx + dy * dy).squareRoot()
        guard length > 0 else { return path }
        let angle = atan2(dy, dx)
        let head = min(16, length * 0.3)
        for side in [-1.0, 1.0] {
            let a = angle + Double.pi - side * 0.5
            path.move(to: to)
            path.addLine(to: CGPoint(x: to.x + cos(a) * head, y: to.y + sin(a) * head))
        }
        return path
    }

    public static func stroke(from: CGPoint, to: CGPoint) -> CGPath {
        let path = CGMutablePath()
        let dx = to.x - from.x, dy = to.y - from.y
        let length = (dx * dx + dy * dy).squareRoot()
        let bow = length > 0 ? 0.04 * length : 0
        let control = CGPoint(
            x: (from.x + to.x) / 2 + (length > 0 ? -dy / length : 0) * bow,
            y: (from.y + to.y) / 2 + (length > 0 ? dx / length : 0) * bow)
        path.move(to: from)
        path.addQuadCurve(to: to, control: control)
        return path
    }

    private static func seed(_ values: Double...) -> Double {
        values.reduce(0) { ($0 * 31 + $1).truncatingRemainder(dividingBy: 997) }
    }
}

/// Subtitle timing: how long a final line stays up without `duration_ms`.
public enum Subtitles {
    public static func readingTime(_ text: String) -> Double {
        min(max(1.6 + 0.055 * Double(text.count), 2.5), 9)
    }
}

/// Holds sketches and enforces the ink limit.
public struct InkCanvas {
    public private(set) var shapeCount = 0

    public init() {}

    public mutating func admit(_ count: Int) throws {
        guard shapeCount + count <= PresenceProtocol.Limits.maxInkShapes else {
            throw PresenceProtocolError("ink limit of \(PresenceProtocol.Limits.maxInkShapes) shapes reached; send clear_ink first")
        }
        shapeCount += count
    }

    public mutating func clear() { shapeCount = 0 }
}
