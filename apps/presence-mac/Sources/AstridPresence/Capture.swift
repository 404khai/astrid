import AppKit
import ImageIO
import PresenceCore
import ScreenCaptureKit
import UniformTypeIdentifiers

/// Where `capture` frames come from. `synthetic` needs no permission and is
/// what tests and development use; `screen` reads the real display.
enum CaptureSource: String {
    case screen
    case synthetic
}

/// ADR 0027: one display at 1x point resolution (frame pixels are display
/// points), Astrid's own overlay excluded, 8-bit sRGB PNG within ADR 0022 limits.
/// Nothing is written to disk; the PNG goes only to stdout for the runtime.
/// A categorical capture failure reported to the runtime.
struct CaptureFailure: Error {
    var reason: String
}

enum FrameCapture {
    struct Frame {
        var png: Data
        var width: Int
        var height: Int
        /// ADR 0038: a coarse in-memory copy for "Do it" freshness checks only.
        var fingerprint: Fingerprint
    }

    static func capture(display: DisplayInfo, screen: NSScreen, source: CaptureSource) async -> Result<Frame, CaptureFailure> {
        let width = Int(display.frame.width.rounded()), height = Int(display.frame.height.rounded())
        if let problem = FrameLimits.check(width: width, height: height, bytes: 0) { return .failure(CaptureFailure(reason: problem)) }
        let image: CGImage
        switch source {
        case .synthetic:
            guard let synthetic = SyntheticFrame.render(width: width, height: height) else { return .failure(CaptureFailure(reason: "render_failed")) }
            image = synthetic
        case .screen:
            switch await screenImage(screen: screen, width: width, height: height) {
            case let .success(captured): image = captured
            case let .failure(failure): return .failure(failure)
            }
        }
        guard let (png, fingerprint) = encode(image, width: width, height: height) else { return .failure(CaptureFailure(reason: "encode_failed")) }
        if let problem = FrameLimits.check(width: width, height: height, bytes: png.count) { return .failure(CaptureFailure(reason: problem)) }
        return .success(Frame(png: png, width: width, height: height, fingerprint: fingerprint))
    }

    private static func screenImage(screen: NSScreen, width: Int, height: Int) async -> Result<CGImage, CaptureFailure> {
        guard CGPreflightScreenCaptureAccess() else {
            // Shows the system prompt once; the grant takes effect on relaunch.
            _ = CGRequestScreenCaptureAccess()
            return .failure(CaptureFailure(reason: "screen_recording_permission_required"))
        }
        guard let displayID = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? CGDirectDisplayID else {
            return .failure(CaptureFailure(reason: "display_unavailable"))
        }
        do {
            let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            guard let target = content.displays.first(where: { $0.displayID == displayID }) else {
                return .failure(CaptureFailure(reason: "display_unavailable"))
            }
            let pid = ProcessInfo.processInfo.processIdentifier
            let own = content.windows.filter { $0.owningApplication?.processID == pid }
            let filter = SCContentFilter(display: target, excludingWindows: own)
            let configuration = SCStreamConfiguration()
            configuration.width = width
            configuration.height = height
            configuration.showsCursor = false
            let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: configuration)
            return .success(image)
        } catch {
            return .failure(CaptureFailure(reason: "capture_failed"))
        }
    }

    /// Redraws into an exact-size 8-bit RGBA sRGB bitmap so the runtime's strict
    /// PNG validation (8-bit RGB/RGBA, non-interlaced) always applies.
    static func encode(_ image: CGImage, width: Int, height: Int) -> (Data, Fingerprint)? {
        guard let space = CGColorSpace(name: CGColorSpace.sRGB),
              let context = CGContext(
                data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        context.interpolationQuality = .high
        context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
        guard let flattened = context.makeImage(), let pixels = context.data else { return nil }
        let bytes = [UInt8](UnsafeBufferPointer(start: pixels.assumingMemoryBound(to: UInt8.self), count: context.bytesPerRow * height))
        // A bitmap context's first row in memory is the image's top row, so
        // cells line up with display points (y down).
        let fingerprint = Fingerprint(rgba: bytes, width: width, height: height, bytesPerRow: context.bytesPerRow)
        let data = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(data, UTType.png.identifier as CFString, 1, nil) else { return nil }
        CGImageDestinationAddImage(destination, flattened, nil)
        return CGImageDestinationFinalize(destination) ? (data as Data, fingerprint) : nil
    }
}

/// A deterministic fake desktop for development and tests: a window with a
/// toolbar of labelled buttons at fixed fractions of the display.
enum SyntheticFrame {
    static func render(width: Int, height: Int) -> CGImage? {
        guard let space = CGColorSpace(name: CGColorSpace.sRGB),
              let context = CGContext(
                data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        let w = CGFloat(width), h = CGFloat(height)
        NSGraphicsContext.saveGraphicsState()
        defer { NSGraphicsContext.restoreGraphicsState() }
        // Flipped so the layout reads top-down like the protocol's coordinates.
        context.translateBy(x: 0, y: h)
        context.scaleBy(x: 1, y: -1)
        NSGraphicsContext.current = NSGraphicsContext(cgContext: context, flipped: true)
        NSColor(srgbRed: 0.16, green: 0.18, blue: 0.22, alpha: 1).setFill()
        NSRect(x: 0, y: 0, width: w, height: h).fill()
        NSColor(srgbRed: 0.95, green: 0.95, blue: 0.96, alpha: 1).setFill()
        NSBezierPath(roundedRect: NSRect(x: w * 0.1, y: h * 0.1, width: w * 0.8, height: h * 0.75), xRadius: 10, yRadius: 10).fill()
        let font = NSFont.systemFont(ofSize: 15, weight: .medium)
        for target in SyntheticTargets.all(width: Double(w), height: Double(h)) {
            NSColor(srgbRed: 0.27, green: 0.35, blue: 0.98, alpha: 1).setFill()
            NSBezierPath(roundedRect: target.box, xRadius: 6, yRadius: 6).fill()
            let label = NSAttributedString(string: target.label, attributes: [.font: font, .foregroundColor: NSColor.white])
            let size = label.size()
            label.draw(at: NSPoint(x: target.box.midX - size.width / 2, y: target.box.midY - size.height / 2))
        }
        return context.makeImage()
    }
}
