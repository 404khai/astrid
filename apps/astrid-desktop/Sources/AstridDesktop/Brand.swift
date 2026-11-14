import AppKit
import DesktopCore
import SwiftUI

/// Astrid's mark in colour, drawn from the same paths and gradient as the web
/// `AstridLogo` (web/src/components/AstridLogo.tsx): blue arches over a
/// 20 × 16 box, teal eyes.
struct AstridLogoMark: View, Animatable {
    var size: CGFloat
    /// Where the eyes look, in logo units (-1...1 each way).
    var gaze: CGSize = .zero
    /// How open the eyes are, 0...1.
    var blink: CGFloat = 1

    /// Glances and blinks move smoothly instead of jumping.
    var animatableData: AnimatablePair<CGFloat, AnimatablePair<CGFloat, CGFloat>> {
        get { AnimatablePair(blink, AnimatablePair(gaze.width, gaze.height)) }
        set {
            blink = newValue.first
            gaze = CGSize(width: newValue.second.first, height: newValue.second.second)
        }
    }

    private static let top = SVGPath.parse("M19.9531 2.8951H17.7133C17.3275 2.8951 16.9572 2.7432 16.6826 2.4721L14.607 0.4231C14.3324 0.152 13.9621 0 13.5762 0H6.4269C6.0411 0 5.6708 0.152 5.3962 0.4231L3.3206 2.4721C3.046 2.7432 2.6757 2.8951 2.2898 2.8951H0.0501C0.0219 2.8951 0 2.9139 0 2.9421V5.5524C0 5.5775 0.0219 5.5931 0.0501 5.5931H3.1447C3.5321 5.5931 3.9039 5.4398 4.1787 5.1668L6.2 3.1587C6.4749 2.8857 6.8465 2.7324 7.234 2.7324H12.8134C13.2006 2.7324 13.572 2.8854 13.8469 3.1582L15.8714 5.1674C16.1463 5.4401 16.5177 5.5931 16.9049 5.5931H19.9531C19.9812 5.5931 20 5.5775 20 5.5524V2.9421C20 2.9139 19.9812 2.8951 19.9531 2.8951Z")
    private static let bottom = SVGPath.parse("M18.5665 10.3552C18.5665 10.3492 18.5616 10.3443 18.5556 10.3443H16.929C16.5424 10.3443 16.1714 10.4969 15.8967 10.769L13.8498 12.796C13.5751 13.068 13.2041 13.2207 12.8174 13.2207H7.2411C6.855 13.2207 6.4845 13.0685 6.2098 12.7971L4.1782 10.7897C3.9036 10.5184 3.5331 10.3662 3.1471 10.3662H0.0532C0.0219 10.3662 0.0031 10.385 0.0031 10.41V13.0203C0.0031 13.0454 0.0219 13.061 0.0532 13.061H2.2835C2.6712 13.061 3.0432 13.2145 3.3181 13.488L5.3955 15.5543C5.6704 15.8277 6.0424 15.9812 6.4301 15.9812H13.5748C13.9615 15.9812 14.3325 15.8286 14.6072 15.5565L16.6979 13.4858C16.9726 13.2137 17.3437 13.061 17.7303 13.061H19.9468C19.975 13.061 19.9969 13.0454 19.9969 13.0203V10.41C19.9969 10.385 19.975 10.3662 19.9468 10.3662H18.5775C18.5714 10.3662 18.5665 10.3613 18.5665 10.3552Z")
    private static let blue = Gradient(stops: [
        .init(color: Color(hex: "#4459F9"), location: 0),
        .init(color: Color(hex: "#1A32EC"), location: 0.5),
        .init(color: Color(hex: "#4459F9"), location: 1),
    ])

    private static func path(_ commands: [SVGPath.Command], scale s: CGFloat) -> Path {
        var path = Path()
        for command in commands {
            switch command {
            case let .move(x, y): path.move(to: CGPoint(x: x * s, y: y * s))
            case let .line(x, y): path.addLine(to: CGPoint(x: x * s, y: y * s))
            case let .cubic(x1, y1, x2, y2, x, y):
                path.addCurve(to: CGPoint(x: x * s, y: y * s), control1: CGPoint(x: x1 * s, y: y1 * s), control2: CGPoint(x: x2 * s, y: y2 * s))
            case .close: path.closeSubpath()
            }
        }
        return path
    }

    var body: some View {
        Canvas { context, canvas in
            let s = canvas.width / 20
            context.fill(Self.path(Self.top, scale: s),
                         with: .linearGradient(Self.blue, startPoint: CGPoint(x: 3.9374 * s, y: 0.9077 * s), endPoint: CGPoint(x: 17.8122 * s, y: 6.7011 * s)))
            context.fill(Self.path(Self.bottom, scale: s),
                         with: .linearGradient(Self.blue, startPoint: CGPoint(x: 2.0595 * s, y: 9.5462 * s), endPoint: CGPoint(x: 15.5399 * s, y: 16.2128 * s)))
            let teal = Color(hex: "#00F7D5")
            for x in [6.0094, 11.0203] {
                let height = 4.93 * max(blink, 0.05)
                let eye = CGRect(x: (x + gaze.width * 1.1) * s, y: (5.4898 + (4.93 - height) / 2 + gaze.height * 0.55) * s, width: 2.99 * s, height: height * s)
                context.fill(Path(roundedRect: eye, cornerRadius: 0.35 * s), with: .color(teal))
            }
        }
        .frame(width: size, height: size * 0.8)
        .accessibilityLabel("Astrid")
    }
}

/// Astrid's mark with living eyes, like the web `AstridLogo`: they follow the
/// pointer over the window, read along as you type in the composer below,
/// glance around on their own when nothing's happening, and blink.
struct TrackingLogoMark: View {
    var size: CGFloat
    /// The composer's text, to read along with.
    var draft = ""
    @State private var gaze: CGSize = .zero
    @State private var blink: CGFloat = 1
    @State private var frame: CGRect = .zero
    @State private var lastPointer: CGPoint = .zero
    @State private var pointerMoved = Date.distantPast
    @State private var typed = Date.distantPast
    @State private var nextGlance = Date()
    @State private var nextBlink = Date().addingTimeInterval(2)
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        AstridLogoMark(size: size, gaze: gaze, blink: blink)
            .animation(.smooth(duration: 0.22), value: gaze)
            .background(GeometryReader { proxy in
                Color.clear
                    .onAppear { frame = proxy.frame(in: .global) }
                    .onChange(of: proxy.frame(in: .global)) { _, next in frame = next }
            })
            .onReceive(Timer.publish(every: 1.0 / 30, on: .main, in: .common).autoconnect()) { now in tick(now) }
            .onChange(of: draft) { _, text in read(text) }
    }

    private func tick(_ now: Date) {
        guard !reduceMotion else { return }
        if now >= nextBlink { blinkOnce(now) }
        guard let window = NSApp.windows.first(where: { $0.isVisible && $0.frame.contains(NSEvent.mouseLocation) }) ?? NSApp.keyWindow,
              let content = window.contentView else { return }
        let point = window.mouseLocationOutsideOfEventStream
        // SwiftUI's global space has its origin at the content's top left.
        let pointer = CGPoint(x: point.x, y: content.bounds.height - point.y)
        if hypot(pointer.x - lastPointer.x, pointer.y - lastPointer.y) > 2 {
            lastPointer = pointer
            pointerMoved = now
        }
        // Typing wins; then a pointer that moved lately; then her own glances.
        if now.timeIntervalSince(typed) < 1.6 { return }
        if now.timeIntervalSince(pointerMoved) < 2.5, window.frame.contains(NSEvent.mouseLocation) {
            let dx = pointer.x - frame.midX, dy = pointer.y - frame.midY
            let distance = max(hypot(dx, dy), 1)
            let reach = min(distance / 220, 1)
            set(CGSize(width: dx / distance * reach, height: dy / distance * reach))
            return
        }
        guard now >= nextGlance else { return }
        nextGlance = now.addingTimeInterval(.random(in: 1.4...3.6))
        if Double.random(in: 0...1) < 0.35 {
            set(.zero)
        } else {
            let angle = Double.random(in: 0...(2 * .pi))
            set(CGSize(width: cos(angle) * .random(in: 0.5...1), height: sin(angle) * .random(in: 0.4...0.9)))
        }
    }

    /// Reads along: down at the composer, left to right through each line.
    private func read(_ text: String) {
        guard !reduceMotion else { return }
        typed = Date()
        guard !text.isEmpty else { return set(CGSize(width: 0, height: 0.6)) }
        let column = Double(text.split(separator: "\n", omittingEmptySubsequences: false).last?.count ?? 0)
        let along = (column.truncatingRemainder(dividingBy: 48)) / 48
        set(CGSize(width: -0.85 + along * 1.7, height: 1))
    }

    private func set(_ next: CGSize) {
        if abs(next.width - gaze.width) > 0.02 || abs(next.height - gaze.height) > 0.02 { gaze = next }
    }

    private func blinkOnce(_ now: Date) {
        let twice = Double.random(in: 0...1) < 0.2
        nextBlink = now.addingTimeInterval(.random(in: 2.5...6))
        withAnimation(.easeIn(duration: 0.07)) { blink = 0.08 }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.09) {
            withAnimation(.easeOut(duration: 0.12)) { blink = 1 }
            guard twice else { return }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.18) {
                withAnimation(.easeIn(duration: 0.07)) { blink = 0.08 }
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.09) { withAnimation(.easeOut(duration: 0.12)) { blink = 1 } }
            }
        }
    }
}

/// Trackpad haptics (Force Touch trackpads only; elsewhere they do nothing).
/// The sidebar ticks as it slides and settles with a firmer bump when it
/// reaches its full width; dragging its edge ticks in detents.
enum Haptics {
    private static var performer: NSHapticFeedbackPerformer { NSHapticFeedbackManager.defaultPerformer }

    static func tick() { performer.perform(.alignment, performanceTime: .now) }
    static func settle() { performer.perform(.levelChange, performanceTime: .now) }

    /// Ticks spread over a slide of `duration`, ending in a settle when it
    /// opens to full width, or a lighter tick when it closes.
    static func slide(opening: Bool, duration: Double = 0.3) {
        let steps = opening ? 4 : 3
        for step in 0..<steps {
            let delay = duration * Double(step) / Double(steps)
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) { tick() }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + duration) { opening ? settle() : tick() }
    }
}
