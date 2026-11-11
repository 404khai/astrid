import CoreGraphics
import Foundation

/// The bundled tour, generated for the connected displays so every point is
/// on screen. `--print-demo` writes it as JSON lines for editing.
public enum DemoScript {
    public static func events(for layout: DisplayLayout) -> [PresenceEvent] {
        guard let primary = layout.displays.first else { return [] }
        let w = primary.frame.width, h = primary.frame.height
        func at(_ fx: Double, _ fy: Double, _ display: DisplayInfo) -> PresencePoint {
            PresencePoint(x: (display.frame.width * fx).rounded(), y: (display.frame.height * fy).rounded(), display: display.index)
        }
        var events: [PresenceEvent] = [
            .show(at: at(0.5, 0.45, primary)),
            .say(text: "Hi, I'm Astrid. This is a scripted presence demo: no model, no screen capture, no clicks.", final: true, durationMs: 3600),
            .wait(ms: 3800),
            .pointAt(at(0.22, 0.28, primary)),
            .wait(ms: 1200),
            .sketch(shapes: [.circle(center: CGPoint(x: (w * 0.22).rounded(), y: (h * 0.28).rounded()), radius: 70)], display: 0),
            .say(text: "I point with my own cursor and circle things in ink.", final: true, durationMs: 3000),
            .wait(ms: 3200),
            .pointAt(at(0.78, 0.30, primary)),
            .wait(ms: 1200),
            .sketch(shapes: [.arrow(
                from: CGPoint(x: (w * 0.62).rounded(), y: (h * 0.52).rounded()),
                to: CGPoint(x: (w * 0.76).rounded(), y: (h * 0.34).rounded()))], display: 0),
            .say(text: "Arrows show where to look next.", final: true, durationMs: 2600),
            .wait(ms: 2800),
            .pointAt(at(0.30, 0.72, primary)),
            .wait(ms: 1100),
            .sketch(shapes: [.line(
                from: CGPoint(x: (w * 0.24).rounded(), y: (h * 0.72 + 14).rounded()),
                to: CGPoint(x: (w * 0.42).rounded(), y: (h * 0.72 + 10).rounded()))], display: 0),
            .say(text: "Subtitles follow along", final: false, durationMs: nil),
            .wait(ms: 700),
            .say(text: "Subtitles follow along as I speak, word by word.", final: true, durationMs: 2600),
            .wait(ms: 2900),
        ]
        for display in layout.displays.dropFirst() {
            events += [
                .pointAt(at(0.5, 0.4, display)),
                .wait(ms: 1400),
                .sketch(shapes: [.circle(center: CGPoint(x: (display.frame.width * 0.5).rounded(), y: (display.frame.height * 0.4).rounded()), radius: 90)], display: display.index),
                .say(text: "I can reach every display: this is display \(display.index + 1).", final: true, durationMs: 2600),
                .wait(ms: 2800),
            ]
        }
        events += [
            .clearInk,
            .pointAt(at(0.5, 0.5, primary)),
            .wait(ms: 1300),
            .say(text: "Move your mouse: my eyes follow your pointer. I can't see your screen.", final: true, durationMs: 4200),
            .wait(ms: 4500),
            .hide,
            .say(text: "Hidden. Showing again...", final: true, durationMs: 1400),
            .wait(ms: 1500),
            .show(at: nil),
            .say(text: "That's the tour. Quit with Ctrl-C or the Astrid menu-bar icon.", final: true, durationMs: 4000),
            .wait(ms: 4200),
        ]
        return events
    }
}
