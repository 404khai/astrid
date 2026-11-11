import CoreGraphics
import Foundation

/// Minimal SVG path-data parser for the shipped mark and cursor geometry:
/// M/L/H/V/C/Z, absolute and relative, with implicit repeats.
public enum SVGPath {
    public static func parse(_ d: String) throws -> CGPath {
        let path = CGMutablePath()
        var tokens = Tokenizer(d)
        var command: Character?
        var current = CGPoint.zero
        var subpathStart = CGPoint.zero

        while let token = tokens.peek() {
            if case let .command(c) = token {
                tokens.next()
                command = c
            } else if command == nil {
                throw PresenceProtocolError("path data must start with a command")
            }
            guard let c = command else { break }
            let relative = c.isLowercase
            func base(_ p: CGPoint) -> CGPoint {
                relative ? CGPoint(x: current.x + p.x, y: current.y + p.y) : p
            }
            switch c.lowercased() {
            case "m":
                let p = base(try tokens.point())
                path.move(to: p)
                current = p
                subpathStart = p
                // Further coordinate pairs after a moveto are linetos.
                command = relative ? "l" : "L"
            case "l":
                let p = base(try tokens.point())
                path.addLine(to: p)
                current = p
            case "h":
                let x = try tokens.number()
                current = CGPoint(x: relative ? current.x + x : x, y: current.y)
                path.addLine(to: current)
            case "v":
                let y = try tokens.number()
                current = CGPoint(x: current.x, y: relative ? current.y + y : y)
                path.addLine(to: current)
            case "c":
                let c1 = base(try tokens.point())
                let c2 = base(try tokens.point())
                let p = base(try tokens.point())
                path.addCurve(to: p, control1: c1, control2: c2)
                current = p
            case "z":
                path.closeSubpath()
                current = subpathStart
                command = nil
                if let next = tokens.peek(), case .number = next {
                    throw PresenceProtocolError("unexpected number after Z")
                }
            default:
                throw PresenceProtocolError("unsupported path command \(c)")
            }
        }
        return path
    }

    private enum Token: Equatable {
        case command(Character)
        case number(Double)
    }

    private struct Tokenizer {
        private let chars: [Character]
        private var index = 0
        private var lookahead: Token??

        init(_ text: String) { chars = Array(text) }

        mutating func peek() -> Token? {
            if let lookahead { return lookahead }
            let token = scan()
            lookahead = .some(token)
            return token
        }

        @discardableResult
        mutating func next() -> Token? {
            let token = peek()
            lookahead = nil
            return token
        }

        mutating func number() throws -> Double {
            guard case let .number(value) = next() else {
                throw PresenceProtocolError("expected a number in path data")
            }
            return value
        }

        mutating func point() throws -> CGPoint {
            CGPoint(x: try number(), y: try number())
        }

        private mutating func scan() -> Token? {
            while index < chars.count, chars[index] == " " || chars[index] == "," || chars[index].isNewline {
                index += 1
            }
            guard index < chars.count else { return nil }
            let c = chars[index]
            if c.isLetter && c != "e" && c != "E" {
                index += 1
                return .command(c)
            }
            var text = ""
            var seenDot = false
            var seenExponent = false
            while index < chars.count {
                let ch = chars[index]
                if ch.isNumber {
                    text.append(ch)
                } else if (ch == "-" || ch == "+") && (text.isEmpty || text.last == "e" || text.last == "E") {
                    text.append(ch)
                } else if ch == "." && !seenDot && !seenExponent {
                    seenDot = true
                    text.append(ch)
                } else if (ch == "e" || ch == "E") && !seenExponent && !text.isEmpty {
                    seenExponent = true
                    text.append(ch)
                } else {
                    break
                }
                index += 1
            }
            guard let value = Double(text) else {
                index = chars.count
                return nil
            }
            return .number(value)
        }
    }
}

/// Astrid's shipped geometry, copied from `web/src/components/AstridLogo.tsx`
/// and the "Arrow click" cursor in `web/src/components/cursors.tsx`.
public enum AstridArt {
    // MARK: Mark (viewBox 0 0 20 16)

    public static let markViewBox = CGSize(width: 20, height: 16)

    public static let upperArch = "M19.9531 2.89855H17.1111L14.1784 0H5.82473L2.89202 2.89855H0.0500782C0.0219092 2.89855 0 2.91735 0 2.94555V5.55895C0 5.58402 0.0219092 5.59969 0.0500782 5.59969H3.74961L6.62911 2.73561H13.4178L16.3005 5.59969H19.9531C19.9812 5.59969 20 5.58402 20 5.55895V2.94555C20 2.91735 19.9812 2.89855 19.9531 2.89855Z"
    public static let lowerArch = "M18.5666 10.3564H16.3256L13.421 13.2362H6.63854L3.74965 10.3784H0.053252C0.0219531 10.3784 0.00317383 10.3972 0.00317383 10.4222V13.0356C0.00317383 13.0607 0.0219531 13.0764 0.053252 13.0764H2.88893L5.82477 16H14.1784L17.1268 13.0764H19.9468C19.975 13.0764 19.9969 13.0607 19.9969 13.0356V10.4222C19.9969 10.3972 19.975 10.3784 19.9468 10.3784H18.5666V10.3564Z"
    public static let leftEye = "M8.94511 5.49634H6.04997C6.02493 5.49634 6.00928 5.51201 6.00928 5.53081V10.391C6.00928 10.4129 6.02493 10.4317 6.04997 10.4317H8.94511C8.97015 10.4317 8.98893 10.4129 8.98893 10.391V5.53081C8.98893 5.51201 8.97015 5.49634 8.94511 5.49634Z"
    public static let rightEye = "M13.9811 5.49634H11.0578C11.0359 5.49634 11.0203 5.51201 11.0203 5.53081V10.391C11.0203 10.4129 11.0359 10.4317 11.0578 10.4317H13.9811C14.0124 10.4317 14.0375 10.4098 14.0375 10.3816V5.54961C14.0375 5.51827 14.0124 5.49634 13.9811 5.49634Z"

    /// Arch gradient stops (sRGB hex) and endpoints in viewBox units.
    public static let archGradient: [UInt32] = [0x4459F9, 0x1A32EC, 0x4459F9]
    public static let upperGradient = (CGPoint(x: 3.9374, y: 0.908735), CGPoint(x: 17.817, y: 6.69738))
    public static let lowerGradient = (CGPoint(x: 2.05951, y: 9.55739), CGPoint(x: 15.5462, y: 16.2193))
    public static let eyeColor: UInt32 = 0x00F7D5

    // MARK: Cursor (32×32 grid)

    public static let cursorGrid: CGFloat = 32
    /// Rendered cursor size in points (web `TIP`).
    public static let cursorSize: CGFloat = 30
    public static let cursorHotspot = CGPoint(x: 7, y: 6)
    public static let cursorArrow = "M7 6 28 16l-10.5 2.5L13 29Z"
    /// `clickFan(7, 6)`: four click lines fanned around the tip (left,
    /// up-left, up, up-right), matching the web cursor.
    public static let cursorClickLines = clickFan(x: 7, y: 6)
    public static let cursorColor: UInt32 = 0x6575FF
    public static let cursorFillAlpha: CGFloat = 1
    public static let arrowStrokeWidth: CGFloat = 1.7
    public static let clickLineWidth: CGFloat = 1.5

    /// Rays from `inner` to `outer` around a tip at 180°, 225°, 270° and 315°
    /// (y down), rounded to hundredths as on the web.
    public static func clickFan(x: Double, y: Double, inner: Double = 3, outer: Double = 5.5) -> String {
        func n(_ v: Double) -> String {
            let r = (v * 100).rounded() / 100
            return r == r.rounded() ? String(Int(r)) : String(r)
        }
        return [180.0, 225, 270, 315].map { degrees in
            let a = degrees * .pi / 180
            return "M\(n(x + cos(a) * inner)) \(n(y + sin(a) * inner))L\(n(x + cos(a) * outer)) \(n(y + sin(a) * outer))"
        }.joined()
    }

    // MARK: Ink and subtitles (web `.ink-layer`, `.subtitle`)

    public static let inkColor: UInt32 = 0x6575FF
    public static let inkWidth: CGFloat = 2.6
    public static let inkDrawDuration = 0.75
    public static let inkStagger = 0.2
}
