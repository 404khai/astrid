import CoreGraphics
import Foundation

/// The fixed set of cursor styles Astrid can point with (`point_at`'s
/// `cursor`). Each one says what to do at the target. The art is copied from
/// `web/src/components/cursors.tsx` (the docs' Cursors page), on the same
/// 32×32 grid. Presentation only: no style clicks, types or drags anything.
public enum CursorStyle: String, CaseIterable, Sendable {
    /// Astrid's own cursor ("Arrow click"): point here. The default.
    case arrowClick = "arrow_click"
    /// "Arrow": plain pointing, no emphasis.
    case arrow
    /// "Click": click this.
    case click
    /// "Pointer": a link or control to choose.
    case pointer
    /// "Press button": press this button.
    case press
    /// "Tap": tap this (trackpad or touch).
    case tap
    /// "Text select": type or select text here.
    case text
    /// "Grab": drag this.
    case grab
    /// "Select": select this region or these items.
    case select
    /// "Crosshair": this exact spot.
    case crosshair
    /// "Focus": look here.
    case focus
    /// "Zoom in": look closer.
    case zoomIn = "zoom_in"
    /// "Zoom out": step back for the bigger picture.
    case zoomOut = "zoom_out"
    /// "Pen": draw or write here.
    case pen
    /// "Not allowed": don't do this here.
    case notAllowed = "not_allowed"

    public static let `default` = CursorStyle.arrowClick

    /// The name on the docs' Cursors page.
    public var docsName: String {
        switch self {
        case .arrowClick: "Arrow click"
        case .arrow: "Arrow"
        case .click: "Click"
        case .pointer: "Pointer"
        case .press: "Press button"
        case .tap: "Tap"
        case .text: "Text select"
        case .grab: "Grab"
        case .select: "Select"
        case .crosshair: "Crosshair"
        case .focus: "Focus"
        case .zoomIn: "Zoom in"
        case .zoomOut: "Zoom out"
        case .pen: "Pen"
        case .notAllowed: "Not allowed"
        }
    }
}

/// One drawn part of a cursor, in grid units.
public struct CursorPart: Equatable, Sendable {
    public enum Paint: Equatable, Sendable {
        /// The cursor colour (web `currentColor`).
        case ink
        /// White glyph strokes drawn over filled ink.
        case white
    }

    public var path: String
    public var fill: Paint?
    public var stroke: Paint?
    public var lineWidth: CGFloat
    public var dash: [CGFloat]
    public var scale: CGFloat

    public static func fill(_ path: String, _ paint: Paint = .ink, scale: CGFloat = 1) -> CursorPart {
        CursorPart(path: path, fill: paint, stroke: nil, lineWidth: 0, dash: [], scale: scale)
    }

    public static func stroke(_ path: String, width: CGFloat = 1.4, _ paint: Paint = .ink, dash: [CGFloat] = []) -> CursorPart {
        CursorPart(path: path, fill: nil, stroke: paint, lineWidth: width, dash: dash, scale: 1)
    }
}

public struct CursorArt: Equatable, Sendable {
    public var hotspot: CGPoint
    public var parts: [CursorPart]
}

extension AstridArt {
    private static let outlineArrow = "M5 4 26 14 15.5 16.5 11 27Z"
    private static let pointerHand =
        "M10 7.5a2 2 0 0 1 4 0V15h.5v-2a2 2 0 0 1 4 0v2h.5v-1a2 2 0 0 1 4 0v1.5h.3a2 2 0 0 1 3.7 1V22c0 4-2.5 7-6.5 7H15c-2.5 0-4-1-5.5-3L4.6 20c-1-1.6.8-3.4 2.4-2.3L10 20Z"

    static func circle(_ cx: Double, _ cy: Double, _ r: Double) -> String {
        "M\(cx - r) \(cy)a\(r) \(r) 0 1 0 \(2 * r) 0a\(r) \(r) 0 1 0 \(-2 * r) 0Z"
    }

    static func rect(_ x: Double, _ y: Double, _ w: Double, _ h: Double, r: Double = 0) -> String {
        guard r > 0 else { return "M\(x) \(y)h\(w)v\(h)h\(-w)Z" }
        return "M\(x + r) \(y)h\(w - 2 * r)a\(r) \(r) 0 0 1 \(r) \(r)v\(h - 2 * r)a\(r) \(r) 0 0 1 \(-r) \(r)"
            + "h\(-(w - 2 * r))a\(r) \(r) 0 0 1 \(-r) \(-r)v\(-(h - 2 * r))a\(r) \(r) 0 0 1 \(r) \(-r)Z"
    }

    /// The web `sparkle(x, y, r)`: short rays above a cursor tip.
    static func sparkle(_ x: Double, _ y: Double, _ r: Double = 3) -> String {
        "M\(x) \(y - r)v-2M\(x - r) \(y - r + 1)l-1.5-1.5M\(x + r) \(y - r + 1)l1.5-1.5M\(x - r - 1) \(y + 1)h-2"
    }

    /// The art for each style, matching `web/src/components/cursors.tsx`.
    public static func cursor(_ style: CursorStyle) -> CursorArt {
        switch style {
        case .arrowClick:
            // Presence's shipped arrow (slightly heavier strokes than the web).
            return CursorArt(hotspot: cursorHotspot, parts: [
                CursorPart(path: cursorArrow, fill: .ink, stroke: .ink, lineWidth: arrowStrokeWidth, dash: [], scale: 1),
                .stroke(cursorClickLines, width: clickLineWidth),
            ])
        case .arrow:
            return CursorArt(hotspot: CGPoint(x: 5, y: 4), parts: [.fill("M5 4 27 14l-11 2.5-6 11Z")])
        case .click:
            return CursorArt(hotspot: CGPoint(x: 11, y: 7), parts: [
                .fill("M11 7 28 25l-6.5.6 4 6.5-3 1.6-4-6.6-4.5 4.4Z", scale: 0.94),
                .stroke(sparkle(8, 6), width: 1.1),
            ])
        case .pointer:
            return CursorArt(hotspot: CGPoint(x: 12, y: 6), parts: [.fill(pointerHand)])
        case .press:
            return CursorArt(hotspot: CGPoint(x: 12, y: 7), parts: [
                .stroke("M7.5 6.5a4.5 4.5 0 0 1 9 0M4.5 6.5a7.5 7.5 0 0 1 15 0", width: 1.1),
                .stroke("M10 7.5a2 2 0 0 1 4 0V16h.5v-2a2 2 0 0 1 4 0v2h.5v-1a2 2 0 0 1 4 0v1.5a2 2 0 0 1 3.5 1.2V23c0 4-2.5 6.5-6.5 6.5H15c-2.4 0-3.8-1-5.3-3l-4.4-6.4c-1-1.6.8-3.3 2.4-2.2L10 20Z"),
            ])
        case .tap:
            return CursorArt(hotspot: CGPoint(x: 12, y: 5), parts: [
                .stroke("M12 2.4V.6M8.6 3.8 7 2.3M15.4 3.8 17 2.3M7.6 6.6H5.4", width: 1.1),
                .fill(pointerHand),
            ])
        case .text:
            return CursorArt(hotspot: CGPoint(x: 16, y: 16), parts: [
                .stroke(rect(3, 10, 26, 12, r: 3), dash: [2, 2.2]),
                .stroke("M13 6c2 0 3 1 3 3v14c0 2 1 3 3 3M19 6c-2 0-3 1-3 3m0 14c0 2-1 3-3 3"),
            ])
        case .grab:
            return CursorArt(hotspot: CGPoint(x: 16, y: 6), parts: [
                .fill("M9 27v-3l-5.5-7.5c-1-1.5 1-3 2.3-1.8L9 18V7.5a1.8 1.8 0 0 1 3.6 0V15V5a1.8 1.8 0 0 1 3.6 0v10V6.3a1.8 1.8 0 0 1 3.6 0V15V9a1.8 1.8 0 0 1 3.6 0v10.5c0 3-1.4 4.5-3 6V27Z"),
            ])
        case .select:
            let handles = [(6.0, 6.0), (16, 6), (26, 6), (6, 16), (26, 16), (6, 26), (16, 26), (26, 26)]
            return CursorArt(hotspot: CGPoint(x: 6, y: 6), parts: [.stroke(rect(6, 6, 20, 20), dash: [2, 2.4])]
                + handles.map { .fill(rect($0.0 - 2, $0.1 - 2, 4, 4)) })
        case .crosshair:
            return CursorArt(hotspot: CGPoint(x: 16, y: 16), parts: [
                .stroke("M16 3v9M16 20v9M3 16h9M20 16h9", width: 1.1),
                .fill(circle(16, 16, 1.4)),
            ])
        case .focus:
            return CursorArt(hotspot: CGPoint(x: 16, y: 16), parts: [
                .stroke("M16 2v8M16 22v8M2 16h8M22 16h8M7 7l4 4M25 7l-4 4M7 25l4-4M25 25l-4-4M11 4l2 5M21 4l-2 5M4 11l5 2M4 21l5-2M28 11l-5 2M28 21l-5-2M11 28l2-5M21 28l-2-5", width: 1),
                .fill(circle(16, 16, 2.2)),
            ])
        case .zoomIn, .zoomOut:
            let glyph = style == .zoomIn ? "M13 8v10M8 13h10" : "M8 13h10"
            return CursorArt(hotspot: CGPoint(x: 13, y: 13), parts: [
                .fill(circle(13, 13, 10)),
                .stroke(glyph, width: 1.6, .white),
                .stroke("M20.5 20.5l7 7"),
            ])
        case .pen:
            return CursorArt(hotspot: CGPoint(x: 5, y: 5), parts: [
                .stroke("M5 5 19 9l4 9-5 5-9-4Z"),
                .fill(rect(11.5, 11.5, 3, 3)),
                .stroke("M5 5l7 7"),
                .fill("M20 25l5-5 3 3-5 5Z"),
            ])
        case .notAllowed:
            return CursorArt(hotspot: CGPoint(x: 5, y: 4), parts: [
                .stroke(outlineArrow),
                .stroke(circle(24, 8, 4), width: 1.1),
                .stroke("M21.3 10.7l5.4-5.4", width: 1.1),
            ])
        }
    }
}
