import AppKit
import PresenceCore
import QuartzCore

/// A transparent, borderless, click-through panel covering one display. It
/// never becomes key or main, joins every Space and sits above full-screen
/// apps. All drawing is Core Animation layers in display-local points.
final class OverlayPanel: NSPanel {
    init(frame: NSRect) {
        super.init(contentRect: frame, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        isOpaque = false
        backgroundColor = .clear
        hasShadow = false
        ignoresMouseEvents = true
        isReleasedWhenClosed = false
        hidesOnDeactivate = false
        isExcludedFromWindowsMenu = true
        level = .screenSaver
        collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary, .ignoresCycle]
        animationBehavior = .none
        setFrame(frame, display: false)
    }

    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

/// The layers for one display.
final class OverlayScene {
    let panel: OverlayPanel
    let display: DisplayInfo
    private let root = CALayer()
    private let inkLayer = CALayer()
    private let subtitle = SubtitleLayer()
    private let logo: LogoLayer
    private let tip: CursorLayer

    init(display: DisplayInfo, cocoaFrame: NSRect) {
        self.display = display
        panel = OverlayPanel(frame: cocoaFrame)
        let scale = CGFloat(display.scale)
        logo = LogoLayer(scale: scale)
        tip = CursorLayer(scale: scale)

        let view = NSView(frame: NSRect(origin: .zero, size: cocoaFrame.size))
        root.frame = view.bounds
        root.isGeometryFlipped = true
        root.masksToBounds = true
        root.contentsScale = scale
        view.layer = root
        view.wantsLayer = true
        panel.contentView = view

        for layer in [inkLayer, subtitle as CALayer] {
            layer.frame = root.bounds
            layer.contentsScale = scale
            root.addSublayer(layer)
        }
        root.addSublayer(logo)
        root.addSublayer(tip)
        subtitle.frame = .zero
        panel.orderFrontRegardless()
    }

    func close() {
        panel.orderOut(nil)
        panel.close()
    }

    func setCursor(_ style: CursorStyle) {
        tip.setStyle(style)
    }

    private func local(_ p: CGPoint) -> CGPoint {
        CGPoint(x: p.x - display.frame.minX, y: p.y - display.frame.minY)
    }

    /// Positions Astrid for this frame; actions disabled by the caller.
    func render(_ motion: PresenceMotion, time: Double, reduceMotion: Bool) {
        let opacity = Float(motion.opacity)
        tip.opacity = opacity
        logo.opacity = opacity
        tip.position = local(motion.tip)
        let origin = local(motion.logo)
        let bob = reduceMotion ? (offset: CGPoint.zero, degrees: 0.0) : PresenceMotion.bob(time: time)
        logo.position = CGPoint(
            x: origin.x + PresenceMotion.logoSize.width / 2 + bob.offset.x,
            y: origin.y + PresenceMotion.logoSize.height / 2 + bob.offset.y)
        logo.setAffineTransform(CGAffineTransform(rotationAngle: bob.degrees * .pi / 180))
        logo.setEyes(offset: motion.eyes, blink: reduceMotion ? 1 : PresenceMotion.blink(time: time))
    }

    /// Renders only this overlay's own layers (never the screen beneath)
    /// over a neutral backdrop, for headless verification.
    func snapshot(backdrop: CGColor) -> CGImage? {
        let scale = CGFloat(display.scale)
        let size = root.bounds.size
        guard let context = CGContext(
            data: nil, width: Int(size.width * scale), height: Int(size.height * scale), bitsPerComponent: 8,
            bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        context.setFillColor(backdrop)
        context.fill(CGRect(x: 0, y: 0, width: size.width * scale, height: size.height * scale))
        // The root is geometry-flipped (y down); flip the bitmap context to match.
        context.translateBy(x: 0, y: size.height * scale)
        context.scaleBy(x: scale, y: -scale)
        root.presentation()?.render(in: context) ?? root.render(in: context)
        return context.makeImage()
    }

    func setSubtitle(_ text: String?, visibleHere: Bool, reduceMotion: Bool) {
        subtitle.update(text: visibleHere ? text : nil, in: root.bounds.size, reduceMotion: reduceMotion)
    }

    func addInk(_ shapes: [InkShape], reduceMotion: Bool, now: CFTimeInterval) {
        let offset = CGAffineTransform(translationX: -display.frame.minX, y: -display.frame.minY)
        for (index, shape) in shapes.enumerated() {
            var transform = offset
            guard let path = InkGeometry.path(for: shape).copy(using: &transform) else { continue }
            guard path.boundingBoxOfPath.insetBy(dx: -20, dy: -20).intersects(root.bounds) else { continue }
            let layer = CAShapeLayer()
            layer.contentsScale = CGFloat(display.scale)
            layer.path = path
            layer.fillColor = nil
            layer.strokeColor = NSColor(hex: AstridArt.inkColor).cgColor
            layer.lineWidth = AstridArt.inkWidth
            layer.lineCap = .round
            layer.lineJoin = .round
            layer.shadowColor = NSColor.black.cgColor
            layer.shadowOpacity = 0.35
            layer.shadowRadius = 2
            layer.shadowOffset = CGSize(width: 0, height: 1)
            if !reduceMotion {
                let draw = CABasicAnimation(keyPath: "strokeEnd")
                draw.fromValue = 0
                draw.toValue = 1
                draw.duration = AstridArt.inkDrawDuration
                draw.beginTime = now + Double(index) * AstridArt.inkStagger
                draw.fillMode = .backwards
                let curve = CubicBezier.ink
                draw.timingFunction = CAMediaTimingFunction(controlPoints: Float(curve.x1), Float(curve.y1), Float(curve.x2), Float(curve.y2))
                layer.add(draw, forKey: "draw")
            }
            inkLayer.addSublayer(layer)
        }
    }

    func clearInk(reduceMotion: Bool) {
        let layers = inkLayer.sublayers ?? []
        guard !layers.isEmpty else { return }
        if reduceMotion {
            layers.forEach { $0.removeFromSuperlayer() }
            return
        }
        CATransaction.begin()
        CATransaction.setAnimationDuration(0.2)
        CATransaction.setCompletionBlock { layers.forEach { $0.removeFromSuperlayer() } }
        layers.forEach { $0.opacity = 0 }
        CATransaction.commit()
    }
}

/// Astrid's "Arrow click" cursor, anchored at its hotspot.
final class CursorLayer: CALayer {
    private(set) var cursorStyle: CursorStyle?

    init(scale: CGFloat) {
        super.init()
        let size = AstridArt.cursorSize
        bounds = CGRect(x: 0, y: 0, width: size, height: size)
        contentsScale = scale
        shadowColor = NSColor.black.cgColor
        shadowOpacity = 0.44
        shadowRadius = 2
        shadowOffset = CGSize(width: 0, height: 2)
        setStyle(.default)
    }

    override init(layer: Any) { super.init(layer: layer) }
    required init?(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    /// Redraws the cursor in one of the docs' styles, with its own hotspot on
    /// the tip position. A no-op when the style is unchanged.
    func setStyle(_ style: CursorStyle) {
        guard style != cursorStyle else { return }
        cursorStyle = style
        let art = AstridArt.cursor(style)
        let unit = AstridArt.cursorSize / AstridArt.cursorGrid
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        let position = self.position
        anchorPoint = CGPoint(x: art.hotspot.x / AstridArt.cursorGrid, y: art.hotspot.y / AstridArt.cursorGrid)
        self.position = position
        sublayers?.forEach { $0.removeFromSuperlayer() }
        let ink = NSColor(hex: AstridArt.cursorColor).cgColor
        let white = NSColor.white.cgColor
        for part in art.parts {
            var transform = CGAffineTransform(scaleX: unit * part.scale, y: unit * part.scale)
            let layer = CAShapeLayer()
            layer.path = (try? SVGPath.parse(part.path))?.copy(using: &transform)
            layer.fillColor = part.fill.map { $0 == .ink ? ink : white }
            layer.strokeColor = part.stroke.map { $0 == .ink ? ink : white }
            layer.lineWidth = part.lineWidth * unit
            layer.lineDashPattern = part.dash.isEmpty ? nil : part.dash.map { NSNumber(value: Double($0 * unit)) }
            layer.lineJoin = .round
            layer.lineCap = .round
            layer.frame = bounds
            layer.contentsScale = contentsScale
            addSublayer(layer)
        }
        CATransaction.commit()
    }
}

/// The Astrid mark: two gradient arches and the eyes, no plate.
final class LogoLayer: CALayer {
    private let gaze = CALayer()
    private let eyes = CAShapeLayer()
    private let unit: CGFloat

    init(scale: CGFloat) {
        let size = PresenceMotion.logoSize
        unit = size.width / AstridArt.markViewBox.width
        super.init()
        bounds = CGRect(origin: .zero, size: size)
        contentsScale = scale
        let transform = CGAffineTransform(scaleX: unit, y: unit)
        // The arches are static, so they are drawn once into an image at
        // backing scale instead of masking gradient layers every frame.
        let arches = CALayer()
        arches.frame = bounds
        arches.contentsScale = scale
        arches.contents = LogoLayer.archImage(size: size, scale: scale)
        addSublayer(arches)
        let eyePath = CGMutablePath()
        for d in [AstridArt.leftEye, AstridArt.rightEye] {
            if let p = try? SVGPath.parse(d) { eyePath.addPath(p, transform: transform) }
        }
        // Blink scales the eyes about their own centre, as `transform-box: fill-box` does on the web.
        let box = eyePath.boundingBoxOfPath
        eyes.bounds = box
        eyes.position = CGPoint(x: box.midX, y: box.midY)
        eyes.path = eyePath
        eyes.fillColor = NSColor(hex: AstridArt.eyeColor).cgColor
        eyes.contentsScale = scale
        gaze.frame = bounds
        gaze.addSublayer(eyes)
        addSublayer(gaze)
    }

    override init(layer: Any) {
        unit = 1
        super.init(layer: layer)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    static func archImage(size: CGSize, scale: CGFloat) -> CGImage? {
        let width = Int(size.width * scale), height = Int(size.height * scale)
        guard let context = CGContext(
            data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
            space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        // Image contents inside the geometry-flipped root are shown flipped,
        // so the y-down viewBox is drawn straight into the y-up bitmap.
        let unit = size.width / AstridArt.markViewBox.width * scale
        context.scaleBy(x: unit, y: unit)
        let colors = AstridArt.archGradient.map { NSColor(hex: $0).cgColor } as CFArray
        guard let gradient = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB), colors: colors, locations: [0, 0.5, 1]) else { return nil }
        for (d, ends) in [(AstridArt.upperArch, AstridArt.upperGradient), (AstridArt.lowerArch, AstridArt.lowerGradient)] {
            guard let path = try? SVGPath.parse(d) else { continue }
            context.saveGState()
            context.addPath(path)
            context.clip()
            context.drawLinearGradient(gradient, start: ends.0, end: ends.1, options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
            context.restoreGState()
        }
        return context.makeImage()
    }

    func setEyes(offset: CGPoint, blink: Double) {
        gaze.setAffineTransform(CGAffineTransform(translationX: offset.x * unit, y: offset.y * unit))
        eyes.setAffineTransform(CGAffineTransform(scaleX: 1, y: blink))
    }
}

/// The subtitle bar: white text on a translucent black pill at the bottom
/// centre of the display Astrid is on. Text is drawn into an image so it
/// renders the same in the window and in `--snapshot`.
final class SubtitleLayer: CALayer {
    private let text = CALayer()
    private var current: String?
    private static let font = NSFont.systemFont(ofSize: 15, weight: .regular)

    override init() {
        super.init()
        backgroundColor = NSColor.black.withAlphaComponent(0.78).cgColor
        cornerRadius = 10
        borderColor = NSColor.white.withAlphaComponent(0.11).cgColor
        borderWidth = 1
        opacity = 0
        addSublayer(text)
    }

    override init(layer: Any) { super.init(layer: layer) }
    required init?(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    func update(text value: String?, in size: CGSize, reduceMotion: Bool) {
        let shown = value.flatMap { $0.isEmpty ? nil : $0 }
        guard shown != current else { return }
        current = shown
        CATransaction.begin()
        CATransaction.setAnimationDuration(0.18)
        guard let shown else {
            CATransaction.setDisableActions(reduceMotion)
            opacity = 0
            CATransaction.commit()
            return
        }
        let paragraph = NSMutableParagraphStyle()
        paragraph.alignment = .center
        paragraph.lineSpacing = 3
        let string = NSAttributedString(string: shown, attributes: [
            .font: Self.font, .foregroundColor: NSColor.white, .paragraphStyle: paragraph,
        ])
        let maxWidth = min(640, size.width * 0.7) - 32
        let options: NSString.DrawingOptions = [.usesLineFragmentOrigin, .usesFontLeading]
        let measured = string.boundingRect(with: CGSize(width: maxWidth, height: 200), options: options)
        let textSize = CGSize(width: ceil(measured.width) + 2, height: min(ceil(measured.height), 66))
        let box = CGSize(width: textSize.width + 32, height: textSize.height + 20)
        CATransaction.setDisableActions(true)
        frame = CGRect(x: (size.width - box.width) / 2, y: size.height - box.height - 28, width: box.width, height: box.height)
        text.frame = CGRect(x: 16, y: 10, width: textSize.width, height: textSize.height)
        text.contentsScale = contentsScale
        text.contents = Self.image(string, size: textSize, scale: contentsScale, options: options)
        CATransaction.setDisableActions(reduceMotion)
        opacity = 1
        CATransaction.commit()
    }

    private static func image(_ string: NSAttributedString, size: CGSize, scale: CGFloat, options: NSString.DrawingOptions) -> CGImage? {
        guard let context = CGContext(
            data: nil, width: Int(size.width * scale), height: Int(size.height * scale), bitsPerComponent: 8,
            bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        // Drawn upside down for the geometry-flipped root (see `archImage`).
        context.translateBy(x: 0, y: size.height * scale)
        context.scaleBy(x: scale, y: -scale)
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(cgContext: context, flipped: false)
        string.draw(with: CGRect(origin: .zero, size: size), options: options)
        NSGraphicsContext.restoreGraphicsState()
        return context.makeImage()
    }
}

extension NSColor {
    convenience init(hex: UInt32) {
        self.init(
            srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
            green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255,
            alpha: 1)
    }
}
