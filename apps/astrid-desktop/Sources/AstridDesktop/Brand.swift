import AppKit
import DesktopCore
import SwiftUI

/// Astrid's mark in colour, drawn from the same paths and gradient as the web
/// `AstridLogo` (web/src/components/AstridLogo.tsx): blue arches over a
/// 20 × 16 box, teal eyes.
struct AstridLogoMark: View {
    var size: CGFloat

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
                let eye = CGRect(x: x * s, y: 5.4898 * s, width: 2.99 * s, height: 4.93 * s)
                context.fill(Path(roundedRect: eye, cornerRadius: 0.35 * s), with: .color(teal))
            }
        }
        .frame(width: size, height: size * 0.8)
        .accessibilityLabel("Astrid")
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
