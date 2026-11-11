import AppKit
import PresenceCore
import QuartzCore

/// Owns the overlay scenes, the event queue and the frame loop. Events come
/// only from the local input source chosen at launch (demo, script file or
/// stdin); there is no network listener. A future Rust runtime sidecar writes
/// the same JSON lines to this process's stdin (ADR 0026).
final class PresenceController: NSObject, NSApplicationDelegate {
    enum Source {
        case demo
        case script([PresenceEvent])
        case stdin
    }

    private let source: Source
    private let forceReduceMotion: Bool
    private let exitAtEnd: Bool
    private let snapshotDirectory: URL?
    private var nextSnapshot = 0.0
    private var snapshotCount = 0

    private var layout = DisplayLayout(displays: [])
    private var scenes: [OverlayScene] = []
    private var player = ScriptPlayer()
    private var motion = PresenceMotion(at: .zero)
    private var canvas = InkCanvas()
    private var ink: [InkShape] = []
    private var subtitle: String?
    private var subtitleClearsAt: Double?
    private var systemReduceMotion = false
    private var lastTick = CACurrentMediaTime()
    private var timer: Timer?
    private var inputEnded = false
    private var statusItem: NSStatusItem?
    private var signalSources: [DispatchSourceSignal] = []

    init(source: Source, forceReduceMotion: Bool, exitAtEnd: Bool, snapshotDirectory: URL?) {
        self.source = source
        self.snapshotDirectory = snapshotDirectory
        self.forceReduceMotion = forceReduceMotion
        self.exitAtEnd = exitAtEnd
    }

    private var reduceMotion: Bool { forceReduceMotion || systemReduceMotion }

    func applicationDidFinishLaunching(_ notification: Notification) {
        systemReduceMotion = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        NSWorkspace.shared.notificationCenter.addObserver(
            self, selector: #selector(accessibilityChanged),
            name: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil)
        NotificationCenter.default.addObserver(
            self, selector: #selector(screensChanged),
            name: NSApplication.didChangeScreenParametersNotification, object: nil)
        installSignalHandlers()
        installStatusItem()
        rebuildScenes()
        if let primary = layout.displays.first {
            motion.jump(to: CGPoint(x: primary.frame.midX, y: primary.frame.midY))
        }
        Output.emit(["type": "ready", "displays": displaysJSON(), "reduce_motion": reduceMotion])

        switch source {
        case .demo:
            player.enqueue(contentsOf: DemoScript.events(for: layout))
            inputEnded = true
        case let .script(events):
            player.enqueue(contentsOf: events)
            inputEnded = true
        case .stdin:
            StdinReader.start { [weak self] item in self?.receive(item) }
        }

        let timer = Timer(timeInterval: 1.0 / 120, repeats: true) { [weak self] _ in self?.tick() }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    // MARK: Input

    private func receive(_ item: StdinReader.Item) {
        switch item {
        case let .event(_, event):
            player.enqueue(event)
        case let .error(line, error):
            Output.error(error.message, line: line)
        case .end:
            inputEnded = true
        }
    }

    // MARK: Frame loop

    private func tick() {
        let now = CACurrentMediaTime()
        let dt = min(now - lastTick, 0.1)
        lastTick = now
        for event in player.drain(now: now) { apply(event, now: now) }

        if let clearAt = subtitleClearsAt, now >= clearAt {
            subtitle = nil
            subtitleClearsAt = nil
        }

        let primaryHeight = NSScreen.screens.first?.frame.height ?? 0
        let pointer = DisplayLayout.cocoaToQuartz(point: NSEvent.mouseLocation, primaryHeight: primaryHeight)
        motion.step(now: now, dt: dt, pointer: pointer, reduceMotion: reduceMotion)

        let subtitleDisplay = layout.display(nearest: motion.tip)?.index
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for scene in scenes {
            scene.render(motion, time: now, reduceMotion: reduceMotion)
        }
        CATransaction.commit()
        for scene in scenes {
            scene.setSubtitle(subtitle, visibleHere: scene.display.index == subtitleDisplay, reduceMotion: reduceMotion)
        }

        if snapshotDirectory != nil, now >= nextSnapshot {
            nextSnapshot = now + 1
            writeSnapshots()
        }

        if exitAtEnd, inputEnded, player.isIdle, !motion.isMoving {
            NSApp.terminate(nil)
        }
    }

    private func apply(_ event: PresenceEvent, now: Double) {
        do {
            switch event {
            case let .show(at):
                if let at { motion.jump(to: try layout.resolve(at)) }
                motion.show(now: now, reduceMotion: reduceMotion)
            case .hide:
                motion.hide(reduceMotion: reduceMotion)
            case let .pointAt(point):
                motion.point(at: try layout.resolve(point), now: now, reduceMotion: reduceMotion)
            case let .say(text, final, durationMs):
                subtitle = text.isEmpty ? nil : text
                if text.isEmpty || !final {
                    subtitleClearsAt = nil
                } else {
                    subtitleClearsAt = now + (durationMs.map { Double($0) / 1000 } ?? Subtitles.readingTime(text))
                }
            case let .sketch(shapes, display):
                let resolved = try InkGeometry.resolve(shapes, display: display, in: layout)
                try canvas.admit(resolved.count)
                ink += resolved
                for scene in scenes { scene.addInk(resolved, reduceMotion: reduceMotion, now: CACurrentMediaTime()) }
            case .clearInk:
                canvas.clear()
                ink.removeAll()
                for scene in scenes { scene.clearInk(reduceMotion: reduceMotion) }
            case .wait:
                break
            case .quit:
                NSApp.terminate(nil)
            }
        } catch let error as PresenceProtocolError {
            Output.error(error.message, line: nil)
        } catch {
            Output.error("\(error)", line: nil)
        }
    }

    /// `--snapshot DIR`: PNGs of Astrid's own layers once a second. The
    /// screen beneath is never captured.
    private func writeSnapshots() {
        guard let directory = snapshotDirectory else { return }
        snapshotCount += 1
        let backdrop = NSColor(srgbRed: 0.42, green: 0.45, blue: 0.5, alpha: 1).cgColor
        for scene in scenes {
            guard let image = scene.snapshot(backdrop: backdrop) else { continue }
            let url = directory.appendingPathComponent(String(format: "display%d-%03d.png", scene.display.index, snapshotCount))
            let rep = NSBitmapImageRep(cgImage: image)
            try? rep.representation(using: .png, properties: [:])?.write(to: url)
        }
    }

    // MARK: Displays

    private func rebuildScenes() {
        scenes.forEach { $0.close() }
        let screens = NSScreen.screens
        layout = DisplayLayout.fromCocoa(frames: screens.map(\.frame), scales: screens.map { Double($0.backingScaleFactor) })
        scenes = zip(layout.displays, screens).map { OverlayScene(display: $0, cocoaFrame: $1.frame) }
        for scene in scenes { scene.addInk(ink, reduceMotion: true, now: CACurrentMediaTime()) }
    }

    private func displaysJSON() -> [[String: Any]] {
        layout.displays.map {
            ["index": $0.index, "x": $0.frame.minX, "y": $0.frame.minY,
             "width": $0.frame.width, "height": $0.frame.height, "scale": $0.scale]
        }
    }

    @objc private func screensChanged() {
        rebuildScenes()
        Output.emit(["type": "displays_changed", "displays": displaysJSON()])
    }

    @objc private func accessibilityChanged() {
        systemReduceMotion = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
    }

    // MARK: Menu bar and quitting

    private func installStatusItem() {
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        item.button?.image = StatusIcon.make()
        item.button?.toolTip = "Astrid presence"
        let menu = NSMenu()
        let title = NSMenuItem(title: "Astrid presence (scripted, no screen access)", action: nil, keyEquivalent: "")
        title.isEnabled = false
        menu.addItem(title)
        menu.addItem(.separator())
        menu.addItem(withTitle: "Show / Hide Astrid", action: #selector(toggleVisible), keyEquivalent: "").target = self
        menu.addItem(withTitle: "Clear Ink", action: #selector(clearInkFromMenu), keyEquivalent: "").target = self
        menu.addItem(.separator())
        menu.addItem(withTitle: "Quit Astrid Presence", action: #selector(quit), keyEquivalent: "q").target = self
        item.menu = menu
        statusItem = item
    }

    @objc private func toggleVisible() {
        if motion.visible { motion.hide(reduceMotion: reduceMotion) } else { motion.show(now: CACurrentMediaTime(), reduceMotion: reduceMotion) }
    }

    @objc private func clearInkFromMenu() { apply(.clearInk, now: CACurrentMediaTime()) }

    @objc private func quit() { NSApp.terminate(nil) }

    private func installSignalHandlers() {
        for sig in [SIGINT, SIGTERM] {
            signal(sig, SIG_IGN)
            let source = DispatchSource.makeSignalSource(signal: sig, queue: .main)
            source.setEventHandler { NSApp.terminate(nil) }
            source.resume()
            signalSources.append(source)
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        timer?.invalidate()
        scenes.forEach { $0.close() }
        Output.emit(["type": "bye"])
    }
}

/// A template menu-bar icon drawn from the mark's arches and eyes.
enum StatusIcon {
    static func make() -> NSImage {
        let size = NSSize(width: 20, height: 16)
        let image = NSImage(size: size, flipped: true) { _ in
            guard let context = NSGraphicsContext.current?.cgContext else { return false }
            context.setFillColor(NSColor.black.cgColor)
            for d in [AstridArt.upperArch, AstridArt.lowerArch, AstridArt.leftEye, AstridArt.rightEye] {
                if let path = try? SVGPath.parse(d) {
                    context.addPath(path)
                    context.fillPath()
                }
            }
            return true
        }
        image.isTemplate = true
        return image
    }
}

/// JSON lines out: status on stdout, errors on stderr.
enum Output {
    static func emit(_ fields: [String: Any]) {
        write(fields, to: FileHandle.standardOutput)
    }

    static func error(_ message: String, line: Int?) {
        var fields: [String: Any] = ["type": "error", "message": message]
        if let line { fields["line"] = line }
        write(fields, to: FileHandle.standardError)
    }

    private static func write(_ fields: [String: Any], to handle: FileHandle) {
        var fields = fields
        fields["v"] = PresenceProtocol.version
        guard let data = try? JSONSerialization.data(withJSONObject: fields, options: [.sortedKeys, .withoutEscapingSlashes]) else { return }
        try? handle.write(contentsOf: data + Data("\n".utf8))
    }
}

/// Reads stdin on a background thread with a bounded line buffer and
/// delivers parsed events on the main queue.
enum StdinReader {
    enum Item {
        case event(line: Int, PresenceEvent)
        case error(line: Int, PresenceProtocolError)
        case end
    }

    static func start(_ deliver: @escaping (Item) -> Void) {
        let thread = Thread {
            var lines = LineAccumulator()
            var number = 0
            var buffer = [UInt8](repeating: 0, count: 4096)
            func handle(_ items: [LineAccumulator.Item]) {
                for item in items {
                    number += 1
                    let result: Item?
                    switch item {
                    case .overflow:
                        result = .error(line: number, PresenceProtocolError("line exceeds \(PresenceProtocol.Limits.maxLineBytes) bytes"))
                    case let .line(raw):
                        let line = raw.trimmingCharacters(in: .whitespacesAndNewlines)
                        if line.isEmpty || line.hasPrefix("#") {
                            result = nil
                        } else {
                            do { result = .event(line: number, try PresenceProtocol.parse(line: line)) } catch let error as PresenceProtocolError {
                                result = .error(line: number, error)
                            } catch {
                                result = .error(line: number, PresenceProtocolError("\(error)"))
                            }
                        }
                    }
                    if let result { DispatchQueue.main.async { deliver(result) } }
                }
            }
            while true {
                let count = read(STDIN_FILENO, &buffer, buffer.count)
                if count <= 0 { break }
                handle(lines.feed(buffer[0..<count]))
            }
            handle(lines.finish())
            DispatchQueue.main.async { deliver(.end) }
        }
        thread.name = "astrid-presence-stdin"
        thread.start()
    }
}
