import CoreGraphics
import Foundation

/// ADR 0022 frame limits, checked by the host before a frame leaves it. The
/// runtime validates again; the host never resizes to fit.
public enum FrameLimits {
    public static let maxDimension = 2560
    public static let maxPixels = 4_096_000
    public static let maxPNGBytes = 1024 * 1024

    /// A categorical failure reason, or nil when the frame is admissible.
    public static func check(width: Int, height: Int, bytes: Int) -> String? {
        if width <= 0 || height <= 0 { return "display_unavailable" }
        if width > maxDimension || height > maxDimension || width * height > maxPixels { return "display_exceeds_frame_limits" }
        if bytes > maxPNGBytes { return "frame_png_exceeds_1mib" }
        return nil
    }
}

/// The labelled buttons drawn in a synthetic frame, in display points.
/// Shared by the renderer and the grounding fixtures so targets are exact.
public enum SyntheticTargets {
    public struct Target: Equatable {
        public var label: String
        public var box: CGRect
    }

    public static func all(width: Double, height: Double) -> [Target] {
        let size = CGSize(width: 120, height: 36)
        func at(_ label: String, _ fx: Double, _ fy: Double) -> Target {
            Target(label: label, box: CGRect(
                x: (width * fx - size.width / 2).rounded(), y: (height * fy - size.height / 2).rounded(),
                width: size.width, height: size.height))
        }
        return [at("Open", 0.2, 0.18), at("Share", 0.5, 0.18), at("Export", 0.8, 0.18), at("Settings", 0.2, 0.75), at("Delete", 0.8, 0.75)]
    }
}
