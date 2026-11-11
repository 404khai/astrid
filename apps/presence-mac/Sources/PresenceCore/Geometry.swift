import CoreGraphics

/// One display in presence space: global points, top-left origin at the
/// primary display, y downward.
public struct DisplayInfo: Equatable, Sendable {
    public var index: Int
    public var frame: CGRect
    public var scale: Double

    public init(index: Int, frame: CGRect, scale: Double = 2) {
        self.index = index
        self.frame = frame
        self.scale = scale
    }
}

/// The current display arrangement. Index 0 is the primary display (the one
/// with the menu bar), matching `NSScreen.screens` order.
public struct DisplayLayout: Equatable, Sendable {
    public var displays: [DisplayInfo]

    public init(displays: [DisplayInfo]) { self.displays = displays }

    /// Builds the layout from AppKit screen frames (bottom-left origin, y up).
    /// The first frame must be the primary display.
    public static func fromCocoa(frames: [CGRect], scales: [Double]? = nil) -> DisplayLayout {
        guard let primary = frames.first else { return DisplayLayout(displays: []) }
        return DisplayLayout(displays: frames.enumerated().map { index, frame in
            DisplayInfo(
                index: index,
                frame: cocoaToQuartz(rect: frame, primaryHeight: primary.height),
                scale: scales?[index] ?? 2)
        })
    }

    public static func cocoaToQuartz(point: CGPoint, primaryHeight: CGFloat) -> CGPoint {
        CGPoint(x: point.x, y: primaryHeight - point.y)
    }

    public static func cocoaToQuartz(rect: CGRect, primaryHeight: CGFloat) -> CGRect {
        CGRect(x: rect.minX, y: primaryHeight - rect.maxY, width: rect.width, height: rect.height)
    }

    /// The union of all displays.
    public var bounds: CGRect {
        displays.map(\.frame).reduce(CGRect.null) { $0.union($1) }
    }

    /// Resolves a protocol point to global presence space.
    public func resolve(_ point: PresencePoint) throws -> CGPoint {
        try resolve(CGPoint(x: point.x, y: point.y), display: point.display)
    }

    public func resolve(_ point: CGPoint, display: Int?) throws -> CGPoint {
        guard let display else { return point }
        guard displays.indices.contains(display) else {
            throw PresenceProtocolError("display \(display) is not connected (\(displays.count) available)")
        }
        let origin = displays[display].frame.origin
        return CGPoint(x: origin.x + point.x, y: origin.y + point.y)
    }

    /// The display containing a global point, else the nearest one.
    public func display(nearest point: CGPoint) -> DisplayInfo? {
        if let hit = displays.first(where: { $0.frame.contains(point) }) { return hit }
        return displays.min { distance(point, $0.frame) < distance(point, $1.frame) }
    }

    private func distance(_ p: CGPoint, _ rect: CGRect) -> CGFloat {
        let dx = max(rect.minX - p.x, 0, p.x - rect.maxX)
        let dy = max(rect.minY - p.y, 0, p.y - rect.maxY)
        return (dx * dx + dy * dy).squareRoot()
    }
}
