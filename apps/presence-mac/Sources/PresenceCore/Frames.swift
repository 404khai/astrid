import CoreGraphics
import ImageIO
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

/// Encodes a frame as an 8-bit RGB PNG (no alpha) at its exact size, so a real
/// screen fits the 1 MiB frame limit without resizing: if the full-colour PNG
/// is too big, colours are quantized step by step (6, 5, then 4 bits per
/// channel), which keeps every coordinate exact and text legible while photos
/// and gradients compress far better. Returns nil if even 4 bits is too big.
public enum FrameEncoding {
    public static let steps = [8, 6, 5, 4]

    public static func fit(rgba: [UInt8], width: Int, height: Int, bytesPerRow: Int,
                           limit: Int = FrameLimits.maxPNGBytes) -> (png: Data, bits: Int)? {
        for bits in steps {
            guard let png = encode(rgb: rgb(rgba, width: width, height: height, bytesPerRow: bytesPerRow, bits: bits),
                                   width: width, height: height) else { return nil }
            if png.count <= limit { return (png, bits) }
        }
        return nil
    }

    /// Tightly packed RGB, each channel kept to its top `bits` bits (low bits
    /// replicated so white stays white).
    public static func rgb(_ rgba: [UInt8], width: Int, height: Int, bytesPerRow: Int, bits: Int) -> [UInt8] {
        let shift = 8 - bits
        var table = [UInt8](repeating: 0, count: 256)
        for v in 0..<256 {
            let top = v >> shift
            var q = top << shift
            if shift > 0 { q |= (top << shift) >> bits }
            table[v] = UInt8(min(255, q))
        }
        var out = [UInt8](repeating: 0, count: width * height * 3)
        for y in 0..<height {
            for x in 0..<width {
                let i = y * bytesPerRow + x * 4, o = (y * width + x) * 3
                out[o] = table[Int(rgba[i])]
                out[o + 1] = table[Int(rgba[i + 1])]
                out[o + 2] = table[Int(rgba[i + 2])]
            }
        }
        return out
    }

    static func encode(rgb: [UInt8], width: Int, height: Int) -> Data? {
        guard let space = CGColorSpace(name: CGColorSpace.sRGB),
              let provider = CGDataProvider(data: Data(rgb) as CFData),
              let image = CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 24,
                                  bytesPerRow: width * 3, space: space, bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.none.rawValue),
                                  provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent) else { return nil }
        let data = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(data, "public.png" as CFString, 1, nil) else { return nil }
        CGImageDestinationAddImage(destination, image, nil)
        return CGImageDestinationFinalize(destination) ? data as Data : nil
    }
}
