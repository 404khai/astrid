import CoreGraphics
import Foundation

/// CSS-style cubic-bezier timing curve, solved for y at a given x.
public struct CubicBezier: Sendable {
    public let x1, y1, x2, y2: Double

    public init(_ x1: Double, _ y1: Double, _ x2: Double, _ y2: Double) {
        (self.x1, self.y1, self.x2, self.y2) = (x1, y1, x2, y2)
    }

    /// The website's glide curve (`.astrid-cursor` transition).
    public static let glide = CubicBezier(0.65, 0, 0.25, 1)
    /// The website's ink draw-on curve.
    public static let ink = CubicBezier(0.55, 0, 0.25, 1)
    public static let easeInOut = CubicBezier(0.42, 0, 0.58, 1)

    private func sample(_ t: Double, _ a: Double, _ b: Double) -> Double {
        let u = 1 - t
        return 3 * u * u * t * a + 3 * u * t * t * b + t * t * t
    }

    public func callAsFunction(_ x: Double) -> Double {
        if x <= 0 { return 0 }
        if x >= 1 { return 1 }
        // Bisection: monotonic in x for 0 <= x1, x2 <= 1; 40 steps is ~1e-12.
        var lo = 0.0, hi = 1.0, t = x
        for _ in 0..<40 {
            t = (lo + hi) / 2
            if sample(t, x1, x2) < x { lo = t } else { hi = t }
        }
        return sample(t, y1, y2)
    }
}

/// One glide of Astrid's tip along a gentle arc.
public struct Flight: Sendable {
    public let from: CGPoint
    public let to: CGPoint
    public let control: CGPoint
    public let start: Double
    public let duration: Double

    public init(from: CGPoint, to: CGPoint, start: Double) {
        self.from = from
        self.to = to
        self.start = start
        let dx = to.x - from.x, dy = to.y - from.y
        let distance = (dx * dx + dy * dy).squareRoot()
        duration = Flight.duration(distance: distance)
        // Bow the path by 12% of its length, always toward the top of the
        // screen so consecutive glides read as one hand.
        var nx = -dy, ny = dx
        if ny > 0 { nx = -nx; ny = -ny }
        let scale = distance > 0 ? 0.12 : 0
        control = CGPoint(
            x: (from.x + to.x) / 2 + (distance > 0 ? nx / distance : 0) * distance * scale,
            y: (from.y + to.y) / 2 + (distance > 0 ? ny / distance : 0) * distance * scale)
    }

    public static func duration(distance: Double) -> Double {
        min(max(0.45 + distance / 1800, 0.45), 1.15)
    }

    public func position(at now: Double) -> CGPoint {
        let t = CubicBezier.glide(duration > 0 ? (now - start) / duration : 1)
        let u = 1 - t
        return CGPoint(
            x: u * u * from.x + 2 * u * t * control.x + t * t * to.x,
            y: u * u * from.y + 2 * u * t * control.y + t * t * to.y)
    }

    public func isFinished(at now: Double) -> Bool { now - start >= duration }
}

/// Astrid's presence motion: the cursor tip, the trailing logo, eye gaze,
/// idle bob, blink and look-around. Pure state; the host renders it.
public struct PresenceMotion {
    /// Where the logo sits relative to the tip hotspot (web `.cursor-follow`).
    public static let followOffset = CGPoint(x: 22, y: 18)
    /// Logo size in points (web `.cursor-follow .astrid-logo`).
    public static let logoSize = CGSize(width: 30, height: 24)
    public static let followTimeConstant = 0.14
    public static let eyeTimeConstant = 0.06
    public static let fadeDuration = 0.2
    public static let pointerIdleBeforeLooking = 4.0

    public private(set) var tip: CGPoint
    /// The logo's top-left corner before bob.
    public private(set) var logo: CGPoint
    /// Eye offset in logo SVG units (viewBox 20×16).
    public private(set) var eyes: CGPoint = .zero
    public private(set) var opacity: Double = 0
    public private(set) var visible = false
    public private(set) var flight: Flight?

    private var lastPointer: CGPoint?
    private var pointerMovedAt = 0.0

    public init(at point: CGPoint) {
        tip = point
        logo = CGPoint(x: point.x + Self.followOffset.x, y: point.y + Self.followOffset.y)
    }

    public var isMoving: Bool { flight != nil }

    public mutating func point(at target: CGPoint, now: Double, reduceMotion: Bool) {
        if reduceMotion || !visible {
            jump(to: target)
        } else {
            flight = Flight(from: tip, to: target, start: now)
        }
    }

    public mutating func show(now: Double, reduceMotion: Bool) {
        visible = true
        if reduceMotion { opacity = 1 }
    }

    public mutating func hide(reduceMotion: Bool) {
        visible = false
        if reduceMotion { opacity = 0 }
    }

    public mutating func jump(to target: CGPoint) {
        flight = nil
        tip = target
        logo = CGPoint(x: target.x + Self.followOffset.x, y: target.y + Self.followOffset.y)
    }

    /// The logo's centre, where the eyes look from.
    public var logoCenter: CGPoint {
        CGPoint(x: logo.x + Self.logoSize.width / 2, y: logo.y + Self.logoSize.height / 2)
    }

    /// Advances one frame. `pointer` is the user's real pointer in global
    /// presence space, when known.
    public mutating func step(now: Double, dt: Double, pointer: CGPoint?, reduceMotion: Bool) {
        if let active = flight {
            tip = active.position(at: now)
            if active.isFinished(at: now) {
                tip = active.to
                flight = nil
            }
        }
        let target = CGPoint(x: tip.x + Self.followOffset.x, y: tip.y + Self.followOffset.y)
        logo = reduceMotion ? target : approach(logo, target, dt: dt, tau: Self.followTimeConstant)

        let targetOpacity = visible ? 1.0 : 0.0
        if reduceMotion {
            opacity = targetOpacity
        } else {
            let delta = dt / Self.fadeDuration
            opacity = opacity < targetOpacity
                ? min(opacity + delta, targetOpacity) : max(opacity - delta, targetOpacity)
        }

        if let pointer, pointer != lastPointer {
            if lastPointer != nil { pointerMovedAt = now }
            lastPointer = pointer
        }
        let gazeTarget: CGPoint
        if let pointer, reduceMotion || now - pointerMovedAt < Self.pointerIdleBeforeLooking {
            gazeTarget = Self.gaze(from: logoCenter, toward: pointer)
        } else {
            gazeTarget = reduceMotion ? .zero : Self.lookAround(time: now)
        }
        eyes = reduceMotion ? gazeTarget : approach(eyes, gazeTarget, dt: dt, tau: Self.eyeTimeConstant)
    }

    private func approach(_ value: CGPoint, _ target: CGPoint, dt: Double, tau: Double) -> CGPoint {
        let k = 1 - exp(-max(dt, 0) / tau)
        return CGPoint(x: value.x + (target.x - value.x) * k, y: value.y + (target.y - value.y) * k)
    }

    // MARK: Character animation (mirrors web/src/styles.css)

    /// Eye travel toward a point, bounded so the eyes stay inside the mark.
    public static func gaze(from center: CGPoint, toward pointer: CGPoint) -> CGPoint {
        CGPoint(x: tanh((pointer.x - center.x) / 180) * 1.2, y: tanh((pointer.y - center.y) / 180) * 1.0)
    }

    /// `follow-bob`: 2.4 s, peaking at translate(3, 3) rotate(-4°).
    public static func bob(time: Double) -> (offset: CGPoint, degrees: Double) {
        let phase = CubicBezier.easeInOut(triangle(time / 2.4))
        return (CGPoint(x: 3 * phase, y: 3 * phase), -4 * phase)
    }

    /// `eyes-blink`: vertical eye scale over a 9 s loop, two quick blinks.
    public static func blink(time: Double) -> Double {
        keyframes(fraction(time / 9), [(0, 1), (0.74, 1), (0.77, 0.05), (0.80, 1), (0.85, 0.05), (0.88, 1), (1, 1)])
    }

    /// `eyes-look`: up, left, right, then rest, over 9 s.
    public static func lookAround(time: Double) -> CGPoint {
        let f = fraction(time / 9)
        let x = keyframes(f, [(0, 0), (0.24, 0), (0.30, -1.2), (0.40, -1.2), (0.46, 1.2), (0.56, 1.2), (0.62, 0), (1, 0)])
        let y = keyframes(f, [(0, 0), (0.08, 0), (0.14, -1), (0.24, -1), (0.30, 0), (1, 0)])
        return CGPoint(x: x, y: y)
    }

    private static func fraction(_ x: Double) -> Double { x - floor(x) }

    /// 0 → 1 → 0 over one period.
    private static func triangle(_ x: Double) -> Double {
        let f = fraction(x)
        return f < 0.5 ? f * 2 : (1 - f) * 2
    }

    /// Piecewise ease-in-out interpolation between sorted keyframes.
    public static func keyframes(_ t: Double, _ keys: [(Double, Double)]) -> Double {
        guard let first = keys.first, let last = keys.last else { return 0 }
        if t <= first.0 { return first.1 }
        if t >= last.0 { return last.1 }
        for (a, b) in zip(keys, keys.dropFirst()) where t >= a.0 && t <= b.0 {
            let span = b.0 - a.0
            let local = span > 0 ? (t - a.0) / span : 1
            return a.1 + (b.1 - a.1) * CubicBezier.easeInOut(local)
        }
        return last.1
    }
}
