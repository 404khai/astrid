import AppKit
import Metal
import QuartzCore

/// Presence's intro: the first time Astrid comes on screen, a colourful wave
/// (Libraries.dev Distortion Waves, vertical type) rises from the bottom middle
/// of the display to the top, leaves its colour on the screen's edges, and goes.
///
/// It is drawn in its own click-through panel above everything, the band by a
/// Metal pass and the sparks as small layers, all from the vendored wave model.
/// Presentation only: it reads nothing on screen (no lens or blur over the
/// apps beneath), takes no input and closes itself when the wave has cleared.
final class IntroWave {
    private let panel: NSPanel
    private let metalLayer = CAMetalLayer()
    private let device: MTLDevice
    private let queue: MTLCommandQueue
    private let pipeline: MTLRenderPipelineState
    private let style: DistortionWaveStyle
    private let source: CGRect
    private let size: CGSize
    private let particles: [WaveParticle]
    private var dots: [CALayer] = []
    private var displayLink: CADisplayLink?
    private var start: CFTimeInterval = 0
    private let done: () -> Void

    /// Keeps the running wave alive until it finishes.
    private static var current: IntroWave?

    /// Plays the wave once on `screen`. Does nothing without Metal; the caller
    /// skips it under Reduce Motion.
    static func play(on screen: NSScreen, done: @escaping () -> Void = {}) {
        guard current == nil else { return }
        do {
            current = try IntroWave(screen: screen) {
                current = nil
                done()
            }
            current?.begin()
        } catch {
            FileHandle.standardError.write(Data("astrid-presence: intro wave unavailable: \(error)\n".utf8))
            done()
        }
    }

    private init(screen: NSScreen, done: @escaping () -> Void) throws {
        guard let device = MTLCreateSystemDefaultDevice(), let queue = device.makeCommandQueue() else {
            throw IntroWaveError.noMetal
        }
        self.device = device
        self.queue = queue
        self.done = done
        // The library's shader, compiled now by the Metal framework (no offline
        // Metal toolchain needed to build presence).
        let library = try device.makeLibrary(source: waveBandShaderSource, options: nil)
        let descriptor = MTLRenderPipelineDescriptor()
        descriptor.vertexFunction = library.makeFunction(name: "waveVertex")
        descriptor.fragmentFunction = library.makeFunction(name: "waveBandFragment")
        descriptor.colorAttachments[0].pixelFormat = .bgra8Unorm
        pipeline = try device.makeRenderPipelineState(descriptor: descriptor)

        var style = DistortionWaveStyle.vertical
        // A Mac display's corners are far tighter than a phone's.
        style.cornerRadius = 12
        self.style = style
        size = screen.frame.size
        // The wave opens from a pill at the bottom middle, as from a button there.
        source = CGRect(x: size.width / 2 - 130, y: size.height - 92, width: 260, height: 48)
        particles = (0..<style.particleCount).map { WaveParticle(index: $0, style: style) }

        panel = NSPanel(contentRect: screen.frame, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.ignoresMouseEvents = true
        panel.level = .screenSaver
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary, .ignoresCycle]
        panel.isReleasedWhenClosed = false
        panel.setFrame(screen.frame, display: false)

        let view = NSView(frame: NSRect(origin: .zero, size: size))
        view.wantsLayer = true
        let scale = screen.backingScaleFactor
        metalLayer.device = device
        metalLayer.pixelFormat = .bgra8Unorm
        metalLayer.isOpaque = false
        metalLayer.framebufferOnly = true
        metalLayer.colorspace = CGColorSpace(name: CGColorSpace.sRGB)
        metalLayer.contentsScale = scale
        metalLayer.frame = view.bounds
        metalLayer.drawableSize = CGSize(width: size.width * scale, height: size.height * scale)
        view.layer?.addSublayer(metalLayer)
        for _ in particles {
            let dot = CALayer()
            dot.backgroundColor = NSColor.white.cgColor
            dot.contentsScale = scale
            dot.opacity = 0
            view.layer?.addSublayer(dot)
            dots.append(dot)
        }
        panel.contentView = view
    }

    private func begin() {
        panel.orderFrontRegardless()
        start = CACurrentMediaTime()
        let link = panel.contentView!.displayLink(target: self, selector: #selector(frame(_:)))
        link.add(to: .main, forMode: .common)
        displayLink = link
    }

    @objc private func frame(_ link: CADisplayLink) {
        let progress = (CACurrentMediaTime() - start) / style.duration
        guard progress < 1 else { return finish() }
        let front = WaveFront(progress: progress, source: source, size: size, style: style)
        render(front)
        moveSparks(front)
    }

    private func render(_ front: WaveFront) {
        guard let drawable = metalLayer.nextDrawable(), let buffer = queue.makeCommandBuffer() else { return }
        Self.encode(front, into: drawable.texture, buffer: buffer, pipeline: pipeline, style: style, size: size, scale: Float(metalLayer.contentsScale))
        buffer.present(drawable)
        buffer.commit()
    }

    /// One frame of the band into `texture`, cleared to transparent.
    private static func encode(_ front: WaveFront, into texture: MTLTexture, buffer: MTLCommandBuffer, pipeline: MTLRenderPipelineState, style: DistortionWaveStyle, size: CGSize, scale: Float) {
        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = texture
        pass.colorAttachments[0].loadAction = .clear
        pass.colorAttachments[0].clearColor = MTLClearColor(red: 0, green: 0, blue: 0, alpha: 0)
        pass.colorAttachments[0].storeAction = .store
        guard let encoder = buffer.makeRenderCommandEncoder(descriptor: pass) else { return }
        let palette = style.bandPalette
        let lumps = WaveRim.lumps(time: front.time, size: size, cornerRadius: style.cornerRadius, pulse: style.rimPulse)
        var params = BandParams(
            scale: scale, paletteCount: Int32(palette.count), lumpCount: Int32(lumps.count),
            lead: Float(style.lead), tail: Float(style.tail), strength: Float(style.strength),
            rimWidth: Float(style.rimWidth), rimSoftness: Float(style.rimSoftness), cornerRadius: Float(style.cornerRadius),
            unevenness: Float(style.rimUnevenness), pulse: Float(style.rimPulse),
            hairWidth: Float(style.hairlineWidth), hairOpacity: Float(style.hairlineOpacity), hairWhite: Float(style.hairlineWhite)
        )
        let field = front.packed
        encoder.setRenderPipelineState(pipeline)
        encoder.setFragmentBytes(&params, length: MemoryLayout<BandParams>.stride, index: 0)
        field.withUnsafeBytes { encoder.setFragmentBytes($0.baseAddress!, length: $0.count, index: 1) }
        palette.withUnsafeBytes { encoder.setFragmentBytes($0.baseAddress!, length: $0.count, index: 2) }
        lumps.withUnsafeBytes { encoder.setFragmentBytes($0.baseAddress!, length: $0.count, index: 3) }
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder.endEncoding()
    }

    /// The sparks: flat white dots that fall toward the source, then ride out
    /// with the front and fade. Positions are from the top left; layers are not.
    private func moveSparks(_ front: WaveFront) {
        let live = front.time > 0
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for (particle, dot) in zip(particles, dots) {
            let opacity = live ? particle.opacity(on: front) : 0
            dot.opacity = Float(opacity)
            guard opacity > 0 else { continue }
            let at = particle.point(on: front, source: source)
            let d = particle.size
            dot.frame = CGRect(x: at.x - d / 2, y: size.height - at.y - d / 2, width: d, height: d)
            dot.cornerRadius = d / 2
        }
        CATransaction.commit()
    }

    /// Debug (ASTRID_WAVE_FRAMES=DIR): renders the wave offscreen at a few
    /// moments to PNGs over a plain dark backdrop, sparks included. Draws
    /// nothing on screen and reads nothing from it.
    static func renderFrames(to directory: URL, size: CGSize = CGSize(width: 1512, height: 982)) throws {
        guard let device = MTLCreateSystemDefaultDevice(), let queue = device.makeCommandQueue() else { throw IntroWaveError.noMetal }
        let library = try device.makeLibrary(source: waveBandShaderSource, options: nil)
        let descriptor = MTLRenderPipelineDescriptor()
        descriptor.vertexFunction = library.makeFunction(name: "waveVertex")
        descriptor.fragmentFunction = library.makeFunction(name: "waveBandFragment")
        descriptor.colorAttachments[0].pixelFormat = .bgra8Unorm
        let pipeline = try device.makeRenderPipelineState(descriptor: descriptor)
        var style = DistortionWaveStyle.vertical
        style.cornerRadius = 12
        let source = CGRect(x: size.width / 2 - 130, y: size.height - 92, width: 260, height: 48)
        let particles = (0..<style.particleCount).map { WaveParticle(index: $0, style: style) }
        let w = Int(size.width), h = Int(size.height)
        let textureDescriptor = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .bgra8Unorm, width: w, height: h, mipmapped: false)
        textureDescriptor.usage = [.renderTarget, .shaderRead]
        textureDescriptor.storageMode = .shared
        guard let texture = device.makeTexture(descriptor: textureDescriptor) else { throw IntroWaveError.noMetal }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        for seconds in [0.3, 0.7, 1.0, 1.4, 2.0, 2.8, 3.4] {
            let front = WaveFront(progress: seconds / style.duration, source: source, size: size, style: style)
            guard let buffer = queue.makeCommandBuffer() else { continue }
            encode(front, into: texture, buffer: buffer, pipeline: pipeline, style: style, size: size, scale: 1)
            buffer.commit()
            buffer.waitUntilCompleted()
            var pixels = [UInt8](repeating: 0, count: w * h * 4)
            texture.getBytes(&pixels, bytesPerRow: w * 4, from: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0)
            let space = CGColorSpace(name: CGColorSpace.sRGB)!
            let info = CGBitmapInfo.byteOrder32Little.rawValue | CGImageAlphaInfo.premultipliedFirst.rawValue
            guard let band = CGContext(data: &pixels, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4, space: space, bitmapInfo: info)?.makeImage(),
                  let out = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0, space: space, bitmapInfo: info) else { continue }
            out.setFillColor(CGColor(srgbRed: 0.16, green: 0.17, blue: 0.2, alpha: 1))
            out.fill(CGRect(x: 0, y: 0, width: w, height: h))
            out.draw(band, in: CGRect(x: 0, y: 0, width: w, height: h))
            out.setFillColor(CGColor(gray: 1, alpha: 1))
            for particle in particles {
                let opacity = particle.opacity(on: front)
                guard opacity > 0 else { continue }
                let at = particle.point(on: front, source: source)
                out.setAlpha(opacity)
                out.fillEllipse(in: CGRect(x: at.x - particle.size / 2, y: size.height - at.y - particle.size / 2, width: particle.size, height: particle.size))
            }
            out.setAlpha(1)
            guard let image = out.makeImage(), let rep = Optional(NSBitmapImageRep(cgImage: image)),
                  let png = rep.representation(using: .png, properties: [:]) else { continue }
            try png.write(to: directory.appendingPathComponent(String(format: "wave-%.1fs.png", seconds)))
        }
    }

    private func finish() {
        displayLink?.invalidate()
        displayLink = nil
        panel.orderOut(nil)
        done()
    }
}

/// Matches `BandParams` in the shader: all 4-byte fields, in order.
private struct BandParams {
    var scale: Float
    var paletteCount: Int32
    var lumpCount: Int32
    var lead: Float
    var tail: Float
    var strength: Float
    var rimWidth: Float
    var rimSoftness: Float
    var cornerRadius: Float
    var unevenness: Float
    var pulse: Float
    var hairWidth: Float
    var hairOpacity: Float
    var hairWhite: Float
}

enum IntroWaveError: Error {
    case noMetal
}
