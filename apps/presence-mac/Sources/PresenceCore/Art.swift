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

    public static let upperArch = "M19.9531 2.8951H17.7133C17.3275 2.8951 16.9572 2.7432 16.6826 2.4721L14.607 0.4231C14.3324 0.152 13.9621 0 13.5762 0H6.4269C6.0411 0 5.6708 0.152 5.3962 0.4231L3.3206 2.4721C3.046 2.7432 2.6757 2.8951 2.2898 2.8951H0.0501C0.0219 2.8951 0 2.9139 0 2.9421V5.5524C0 5.5775 0.0219 5.5931 0.0501 5.5931H3.1447C3.5321 5.5931 3.9039 5.4398 4.1787 5.1668L6.2 3.1587C6.4749 2.8857 6.8465 2.7324 7.234 2.7324H12.8134C13.2006 2.7324 13.572 2.8854 13.8469 3.1582L15.8714 5.1674C16.1463 5.4401 16.5177 5.5931 16.9049 5.5931H19.9531C19.9812 5.5931 20 5.5775 20 5.5524V2.9421C20 2.9139 19.9812 2.8951 19.9531 2.8951Z"
    public static let lowerArch = "M18.5665 10.3552C18.5665 10.3492 18.5616 10.3443 18.5556 10.3443H16.929C16.5424 10.3443 16.1714 10.4969 15.8967 10.769L13.8498 12.796C13.5751 13.068 13.2041 13.2207 12.8174 13.2207H7.2411C6.855 13.2207 6.4845 13.0685 6.2098 12.7971L4.1782 10.7897C3.9036 10.5184 3.5331 10.3662 3.1471 10.3662H0.0532C0.0219 10.3662 0.0031 10.385 0.0031 10.41V13.0203C0.0031 13.0454 0.0219 13.061 0.0532 13.061H2.2835C2.6712 13.061 3.0432 13.2145 3.3181 13.488L5.3955 15.5543C5.6704 15.8277 6.0424 15.9812 6.4301 15.9812H13.5748C13.9615 15.9812 14.3325 15.8286 14.6072 15.5565L16.6979 13.4858C16.9726 13.2137 17.3437 13.061 17.7303 13.061H19.9468C19.975 13.061 19.9969 13.0454 19.9969 13.0203V10.41C19.9969 10.385 19.975 10.3662 19.9468 10.3662H18.5775C18.5714 10.3662 18.5665 10.3613 18.5665 10.3552Z"
    public static let leftEye = "M8.9452 5.4898H6.0501C6.025 5.4898 6.0094 5.5055 6.0094 5.5243V10.3787C6.0094 10.4006 6.025 10.4194 6.0501 10.4194H8.9452C8.9703 10.4194 8.989 10.4006 8.989 10.3787V5.5243C8.989 5.5055 8.9703 5.4898 8.9452 5.4898Z"
    public static let rightEye = "M13.9812 5.4898H11.0579C11.036 5.4898 11.0203 5.5055 11.0203 5.5243V10.3787C11.0203 10.4006 11.036 10.4194 11.0579 10.4194H13.9812C14.0125 10.4194 14.0376 10.3975 14.0376 10.3693V5.543C14.0376 5.5117 14.0125 5.4898 13.9812 5.4898Z"

    /// Arch gradient stops (sRGB hex) and endpoints in viewBox units.
    public static let archGradient: [UInt32] = [0x4459F9, 0x1A32EC, 0x4459F9]
    public static let upperGradient = (CGPoint(x: 3.9374, y: 0.9077), CGPoint(x: 17.8122, y: 6.7011))
    public static let lowerGradient = (CGPoint(x: 2.0595, y: 9.5462), CGPoint(x: 15.5399, y: 16.2128))
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
