import AppKit
import UserNotifications
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
    private let captureSource: CaptureSource
    private var capturesInFlight = 0
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
    /// ADR 0029: resident hosts own timers and stay in the menu bar.
    private let resident: Bool
    private let timersURL: URL
    private var timers = TimerStore()
    private var nextTimerCheck = 0.0

    private let askConfig: AskConfig?
    private var ask: AskController?
    /// ADR 0038: true while an operator session drives the cursor Astrid.
    private var operating = false
    /// The latest look's fingerprint, for "Do it" freshness. Memory only.
    private var lastLook: (id: String, display: Int, at: Date, fingerprint: Fingerprint)?
    /// The intro wave plays once, the first time Astrid comes on screen.
    private let intro: Bool
    private var introPlayed = false

    init(source: Source, forceReduceMotion: Bool, exitAtEnd: Bool, snapshotDirectory: URL?, captureSource: CaptureSource = .screen, resident: Bool = false, timersURL: URL = TimerStore.defaultURL, ask: AskConfig? = nil, intro: Bool = true) {
        self.askConfig = ask
        self.intro = intro
        self.captureSource = captureSource
        self.resident = resident
        self.timersURL = timersURL
        self.source = source
        self.snapshotDirectory = snapshotDirectory
        self.forceReduceMotion = forceReduceMotion
        self.exitAtEnd = exitAtEnd
    }

    private var reduceMotion: Bool { forceReduceMotion || systemReduceMotion }

    /// The Distortion Waves intro on the main display, once per launch; never
    /// under Reduce Motion or with --no-intro.
    private func playIntro() {
        guard !introPlayed else { return }
        introPlayed = true
        playWave()
    }

    /// The same wave on demand (`wave`, e.g. an operator session going live).
    /// Skipped under Reduce Motion or with --no-intro; a running wave is not
    /// restarted. The wave's panel is this process's window, so captures
    /// exclude it like the rest of the overlay.
    func playWave() {
        guard intro, !reduceMotion, let screen = NSScreen.main ?? NSScreen.screens.first else { return }
        IntroWave.play(on: screen)
    }

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
        if let askConfig {
            ask = AskController(config: askConfig)
            ask?.host = self
            AXElements.startTracking()
            // In --ask mode the mark is on screen from launch.
            playIntro()
        }
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
        // ADR 0038: the ask box rides beside the cursor Astrid while it works.
        if operating { ask?.follow(tip: motion.tip, primaryHeight: primaryHeight) }

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

        if resident, now >= nextTimerCheck {
            nextTimerCheck = now + 0.25
            fireDueTimers()
        }

        if exitAtEnd, !resident, inputEnded, player.isIdle, !motion.isMoving, capturesInFlight == 0 {
            NSApp.terminate(nil)
        }
    }

    private func apply(_ event: PresenceEvent, now: Double) {
        do {
            switch event {
            case let .show(at):
                playIntro()
                if let at { motion.jump(to: try layout.resolve(at)) }
                motion.show(now: now, reduceMotion: reduceMotion)
            case .hide:
                motion.hide(reduceMotion: reduceMotion)
                for scene in scenes { scene.setCursor(.default) }
            case let .pointAt(point, cursor):
                let target = try layout.resolve(point)
                for scene in scenes { scene.setCursor(cursor ?? .default) }
                motion.point(at: target, now: now, reduceMotion: reduceMotion)
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
            case .wave:
                playWave()
            case let .capture(requestId, display):
                capture(requestId: requestId, display: display)
            case .timerStart, .timerCancel, .alarmSet, .timersList:
                try applyTimer(event)
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

    // MARK: Capture (ADR 0027)

    /// One frame for the runtime, answered on stdout. The overlay keeps
    /// drawing; ScreenCaptureKit excludes this process's windows.
    private func capture(requestId: String, display index: Int) {
        capturesInFlight += 1
        captureFrame(requestId: requestId, display: index) { reply in
            // A frame line can be over a megabyte; never block drawing on the
            // reader draining the pipe.
            Output.queue.async {
                Output.emit(reply)
                DispatchQueue.main.async { self.capturesInFlight -= 1 }
            }
        }
    }

    /// Captures one display and hands back the `frame` or `capture_failed`
    /// reply, on the main thread.
    private func captureFrame(requestId: String, display index: Int, reply: @escaping ([String: Any]) -> Void) {
        let screens = NSScreen.screens
        guard layout.displays.indices.contains(index), screens.indices.contains(index) else {
            reply(["type": "capture_failed", "request_id": requestId, "reason": "display_unavailable"])
            return
        }
        let display = layout.displays[index], screen = screens[index], source = captureSource
        Task { @MainActor in
            let result = await FrameCapture.capture(display: display, screen: screen, source: source)
            switch result {
            case let .success(frame):
                self.lastLook = (requestId, index, Date(), frame.fingerprint)
                reply(["type": "frame", "request_id": requestId, "display": index,
                       "width": frame.width, "height": frame.height,
                       "png_base64": frame.png.base64EncodedString()])
            case let .failure(failure):
                reply(["type": "capture_failed", "request_id": requestId, "reason": failure.reason])
            }
        }
    }

    // MARK: Displays

    private func rebuildScenes() {
        scenes.forEach { $0.close() }
        let screens = NSScreen.screens
        layout = DisplayLayout.fromCocoa(frames: screens.map(\.frame), scales: screens.map { Double($0.backingScaleFactor) })
        // The ask surface is Astrid's only window in --ask mode: no full-screen overlay.
        // In --ask mode the overlay exists from launch but stays hidden until an
        // operator session shows the cursor Astrid (no panels appear mid-session).
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

    // MARK: Timers and alarms (ADR 0029)

    private func loadTimers() {
        guard resident else { return }
        timers = TimerStore.load(from: timersURL)
        if Bundle.main.bundleIdentifier != nil {
            UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
        }
        // Anything that came due while the host was not running fires now, once.
        fireDueTimers()
    }

    private func applyTimer(_ event: PresenceEvent) throws {
        guard resident else { throw PresenceProtocolError("timers need a resident host (--resident)") }
        let now = Date()
        switch event {
        case let .timerStart(id, seconds, label):
            let entry = try timers.startTimer(id: id, seconds: seconds, label: label, now: now)
            Output.emit(["type": "timer_set", "id": entry.id, "kind": "timer", "fires_at": ISO8601DateFormatter().string(from: entry.fireAt)])
        case let .alarmSet(id, at, label):
            let entry = try timers.setAlarm(id: id, at: at, label: label, now: now)
            Output.emit(["type": "timer_set", "id": entry.id, "kind": "alarm", "fires_at": ISO8601DateFormatter().string(from: entry.fireAt)])
        case let .timerCancel(id):
            guard timers.cancel(id: id) != nil else { throw PresenceProtocolError("no timer \(id)") }
            Output.emit(["type": "timer_cancelled", "id": id])
        case .timersList:
            Output.emit(["type": "timers", "timers": timers.entries.map(timerJSON)])
            return
        default:
            return
        }
        persistTimers()
    }

    private func timerJSON(_ entry: TimerEntry) -> [String: Any] {
        ["id": entry.id, "kind": entry.kind.rawValue, "label": entry.label, "fires_at": ISO8601DateFormatter().string(from: entry.fireAt)]
    }

    private func persistTimers() {
        do { try timers.save(to: timersURL) } catch { Output.error("could not save timers", line: nil) }
        rebuildTimerMenu()
    }

    private func fireDueTimers() {
        let now = Date()
        let due = timers.takeDue(now: now)
        guard !due.isEmpty else { return }
        persistTimers()
        for entry in due {
            let text = timerFiredText(entry, now: now)
            Output.emit(["type": "timer_fired", "id": entry.id, "kind": entry.kind.rawValue, "label": entry.label, "late_seconds": max(0, Int(now.timeIntervalSince(entry.fireAt)))])
            // Show Astrid near the pointer with a subtitle, chime and notify.
            let primaryHeight = NSScreen.screens.first?.frame.height ?? 0
            let pointer = DisplayLayout.cocoaToQuartz(point: NSEvent.mouseLocation, primaryHeight: primaryHeight)
            if !motion.visible { motion.jump(to: CGPoint(x: pointer.x + 40, y: pointer.y + 40)) }
            motion.show(now: CACurrentMediaTime(), reduceMotion: reduceMotion)
            subtitle = text
            subtitleClearsAt = CACurrentMediaTime() + 8
            NSSound(named: "Glass")?.play()
            if Bundle.main.bundleIdentifier != nil {
                let content = UNMutableNotificationContent()
                content.title = entry.kind == .alarm ? "Astrid alarm" : "Astrid timer"
                content.body = text
                content.sound = .default
                UNUserNotificationCenter.current().add(UNNotificationRequest(identifier: "astrid-\(entry.id)", content: content, trigger: nil))
            }
        }
    }

    private var timerMenuItems: [NSMenuItem] = []

    private func rebuildTimerMenu() {
        guard let menu = statusItem?.menu else { return }
        timerMenuItems.forEach { menu.removeItem($0) }
        timerMenuItems = []
        guard resident else { return }
        let header = NSMenuItem(title: timers.entries.isEmpty ? "No timers" : "Timers (click to cancel)", action: nil, keyEquivalent: "")
        header.isEnabled = false
        var items = [header]
        let format = DateFormatter()
        format.timeStyle = .short
        format.dateStyle = .none
        for entry in timers.entries {
            let name = entry.label.isEmpty ? entry.id : entry.label
            let item = NSMenuItem(title: "\(entry.kind == .alarm ? "⏰" : "⏲") \(name) at \(format.string(from: entry.fireAt))", action: #selector(cancelTimerFromMenu(_:)), keyEquivalent: "")
            item.target = self
            item.representedObject = entry.id
            items.append(item)
        }
        items.append(.separator())
        for (offset, item) in items.enumerated() { menu.insertItem(item, at: 2 + offset) }
        timerMenuItems = items
    }

    @objc private func cancelTimerFromMenu(_ sender: NSMenuItem) {
        guard let id = sender.representedObject as? String, timers.cancel(id: id) != nil else { return }
        Output.emit(["type": "timer_cancelled", "id": id])
        persistTimers()
    }

    // MARK: Menu bar and quitting

    private func installStatusItem() {
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        item.button?.image = StatusIcon.make()
        item.button?.toolTip = "Astrid presence"
        let menu = NSMenu()
        let title = NSMenuItem(title: resident ? "Astrid (resident: timers and alarms)" : "Astrid presence (scripted, no screen access)", action: nil, keyEquivalent: "")
        title.isEnabled = false
        menu.addItem(title)
        menu.addItem(.separator())
        menu.addItem(withTitle: "Show / Hide Astrid", action: #selector(toggleVisible), keyEquivalent: "").target = self
        menu.addItem(withTitle: "Clear Ink", action: #selector(clearInkFromMenu), keyEquivalent: "").target = self
        menu.addItem(.separator())
        menu.addItem(withTitle: "Quit Astrid Presence", action: #selector(quit), keyEquivalent: "q").target = self
        item.menu = menu
        statusItem = item
        loadTimers()
        rebuildTimerMenu()
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
        ask?.stop()
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
    /// Serial queue for large writes; keeps stdout lines whole and ordered.
    static let queue = DispatchQueue(label: "astrid-presence-output")

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

// MARK: - Operator (ADR 0038)

extension PresenceController: OperatorHost {
    /// The cursor Astrid takes over from the ask panel's mark, from where it was.
    func operatorBegan(from mark: CGPoint) {
        operating = true
        motion.jump(to: mark)
        player.enqueue(.show(at: nil))
        // The user's app keeps focus, so clicks and typing land there.
        AXElements.targetApp()?.activate()
        // Reading on-screen controls and acting both need Accessibility; asks once.
        _ = Injector.trusted()
    }

    func operatorPresent(_ fields: [String: Any]) {
        guard let data = try? JSONSerialization.data(withJSONObject: fields),
              let line = String(data: data, encoding: .utf8) else { return }
        do {
            let event = try PresenceProtocol.parse(line: line)
            // Only presentation reaches the overlay from the ask child.
            switch event {
            case .pointAt, .sketch, .clearInk, .say, .wave: break
            default: throw PresenceProtocolError("operator may only present")
            }
            player.enqueue(event)
        } catch let error as PresenceProtocolError {
            Output.error("operator: \(error.message)", line: nil)
        } catch {}
    }

    /// One frame plus the target app's Accessibility elements, read at the
    /// same moment on a background queue.
    func operatorCapture(requestId: String, display index: Int, reply: @escaping ([String: Any]) -> Void) {
        guard PresenceProtocol.isRequestId(requestId), layout.displays.indices.contains(index) else {
            return reply(["type": "capture_failed", "request_id": requestId, "reason": "display_unavailable"])
        }
        let display = layout.displays[index], app = AXElements.targetApp()
        var elements: [[Any]]?
        let group = DispatchGroup()
        group.enter()
        DispatchQueue.global(qos: .userInitiated).async {
            elements = AXElements.collect(display: display, app: app)
            group.leave()
        }
        captureFrame(requestId: requestId, display: index) { frame in
            group.notify(queue: .main) {
                var frame = frame
                if frame["type"] as? String == "frame", let elements, !elements.isEmpty { frame["elements"] = elements }
                reply(frame)
            }
        }
    }
    /// "Do it": refuses unless the screen around the target still matches the
    /// look the step was planned on, then posts exactly one action.
    func operatorAct(_ fields: [String: Any], reply: @escaping (String) -> Void) {
        guard let frame = fields["frame"] as? String, let index = (fields["display"] as? NSNumber)?.intValue,
              let x = (fields["x"] as? NSNumber)?.doubleValue, let y = (fields["y"] as? NSNumber)?.doubleValue,
              let raw = fields["action"] as? [String: Any], let action = try? OperatorAction.parse(raw),
              layout.displays.indices.contains(index) else { return reply("invalid_action") }
        let display = layout.displays[index]
        let target = CGPoint(x: x, y: y)
        guard target.x >= 0, target.y >= 0, target.x < display.frame.width, target.y < display.frame.height else {
            return reply("invalid_action")
        }
        guard let look = lastLook, look.id == frame, look.display == index, Date().timeIntervalSince(look.at) < 120 else {
            return reply("stale")
        }
        // Synthetic frames are not the real screen: never post input against them.
        guard captureSource == .screen else { return reply("synthetic_capture_no_input") }
        guard Injector.trusted() else { return reply("accessibility_permission_required") }
        let check = "act-\(UUID().uuidString.prefix(8))"
        captureFrame(requestId: check, display: index) { [weak self] result in
            guard let self else { return }
            guard result["type"] as? String == "frame", let fresh = self.lastLook, fresh.id == check,
                  look.fingerprint.matches(fresh.fingerprint, around: target) else {
                return reply("stale")
            }
            // The planning look stays the reference for this step.
            self.lastLook = look
            let global = CGPoint(x: display.frame.minX + target.x, y: display.frame.minY + target.y)
            Injector.perform(action, at: global)
            reply("done")
        }
    }

    func operatorEnded() {
        guard operating else { return }
        operating = false
        player.enqueue(contentsOf: [.clearInk, .hide])
        lastLook = nil
    }
}
