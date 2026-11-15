import AppKit
import AstridWebUI
import PresenceCore
import WebKit

/// ADR 0033: the presence app's ask surface. A small floating panel in the
/// bottom-right corner hosts the web UI (logo → gooey input → agent states →
/// brief) and relays it to `astrid presence-ask`, a JSON-lines runtime client.
/// Unlike the click-through overlay, this panel takes clicks and typing.
final class AskPanel: NSPanel {
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }
}

struct AskConfig {
    var ui: URL
    var astrid: URL
    var model: String
    var mail: String?
    /// The `astrid-voice` executable (ADR 0035); nil uses macOS's built-in voice.
    var voiceCommand: URL?
    var voiceName = "af_heart"
    /// Wake word ("hey Astrid"): an always-on, on-device listener. Off unless `--wake`.
    var wake = false
    /// ADR 0038: the `/operator` image model (nil: the runtime's default).
    var operatorModel: String?
    /// ADR 0041: Calendar and Reminders through EventKit (`--ask-calendar off` turns it off).
    var calendar = true
}

/// What the operator (ADR 0038) needs from the presence app.
protocol OperatorHost: AnyObject {
    func operatorPresent(_ fields: [String: Any])
    func operatorCapture(requestId: String, display: Int, reply: @escaping ([String: Any]) -> Void)
    func operatorAct(_ fields: [String: Any], reply: @escaping (String) -> Void)
    func operatorBegan(from mark: CGPoint)
    func operatorEnded()
}

final class AskController: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
    private let config: AskConfig
    weak var host: OperatorHost?
    /// ADR 0038: while operator runs the panel's mark rides with the cursor
    /// tip and talks to you from there.
    private(set) var following = false
    /// ADR 0042: nudges wait while an ask runs or the panel is open.
    private var asking = false
    private var expanded = false
    var busy: Bool { asking || following || expanded }
    /// Astrid itself holds the microphone (push-to-talk or the wake word).
    var micActive: Bool { listening || wakeListener != nil }
    weak var nudges: NudgeController?
    /// Writes to the runtime child, off the main thread (frames are large).
    private let childWrites = DispatchQueue(label: "dev.astrid.presence.ask-writes")
    private let panel: AskPanel
    private let web: WKWebView
    private var child: Process?
    private var childInput: FileHandle?
    private var buffer = Data()
    private var contentSize = NSSize(width: 84, height: 76)
    /// Bottom-centre of the panel in screen points. Starts at the centre of the
    /// main screen; dragging moves it. The panel grows upward around it.
    private var anchor: NSPoint = {
        let visible = (NSScreen.main ?? NSScreen.screens.first)?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
        return NSPoint(x: visible.midX, y: visible.midY - 38)
    }()
    private static let margin: CGFloat = 20
    private var pointerTimer: Timer?
    private var lastPointer = NSPoint(x: -1, y: -1)
    private let voice = VoiceInput()
    private var wakeListener: WakeListener?
    /// True while push-to-talk holds the microphone (the wake word is paused).
    private var listening = false
    private var hotkey: Hotkey?
    private var dragTimer: Timer?
    private let speaker: Speaker
    /// ADR 0041: EventKit calls block, so they run one at a time off the main thread.
    private let calendarQueue = DispatchQueue(label: "dev.astrid.presence.calendar")
    private lazy var calendar = CalendarService(source: EventKitCalendar())
    /// ADR 0036: the live canvas, created on the first drawing.
    private lazy var canvas: CanvasController = {
        let canvas = CanvasController(page: config.ui.deletingLastPathComponent().appendingPathComponent("canvas.html"))
        canvas.onVisible = { [weak self] on in self?.deliver(["type": "canvas-open", "on": on]) }
        return canvas
    }()

    init(config: AskConfig) {
        self.config = config
        panel = AskPanel(contentRect: NSRect(x: 0, y: 0, width: 84, height: 76), styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        let configuration = WKWebViewConfiguration()
        // Serve the built UI from its folder over a private scheme so ES module
        // scripts load with a real origin (file:// gives them a null origin).
        configuration.setURLSchemeHandler(UIFiles(root: config.ui.deletingLastPathComponent()), forURLScheme: "astrid-ui")
        web = WKWebView(frame: .zero, configuration: configuration)
        speaker = Speaker(command: config.voiceCommand, voice: config.voiceName)
        super.init()
        configuration.userContentController.add(self, name: "astrid")
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.becomesKeyOnlyIfNeeded = true
        web.setValue(false, forKey: "drawsBackground")
        web.navigationDelegate = self
        panel.contentView = web
        web.load(URLRequest(url: URL(string: "astrid-ui://app/\(config.ui.lastPathComponent)")!))
        place()
        panel.orderFrontRegardless()
        // The mark's eyes follow the system cursor anywhere on screen. Polling
        // NSEvent.mouseLocation needs no permission; only movement is sent.
        let timer = Timer(timeInterval: 1.0 / 30, repeats: true) { [weak self] _ in self?.trackPointer() }
        RunLoop.main.add(timer, forMode: .common)
        pointerTimer = timer
        voice.onFrame = { [weak self] frame in self?.deliver(frame.message) }
        speaker.onFrame = { [weak self] frame in
            var message = frame.message
            message["source"] = "speaker"
            self?.deliver(message)
        }
        speaker.onSpeaking = { [weak self] on in
            // Astrid's own voice must not wake her.
            if on { self?.wakeListener?.pause() } else { self?.wakeListener?.resume() }
            self?.deliver(["type": "speaking", "on": on])
        }
        voice.onTranscript = { [weak self] text, final in self?.deliver(["type": "transcript", "text": text, "final": final]) }
        voice.onError = { [weak self] message in self?.deliver(["type": "voice-error", "message": message]) }
        if config.wake {
            let listener = WakeListener()
            listener.onWake = { [weak self] rest in self?.deliver(["type": "wake", "text": String(rest.prefix(4000))]) }
            listener.onError = { [weak self] message in self?.deliver(["type": "voice-error", "message": message]) }
            listener.start()
            wakeListener = listener
        }
        // Hold ⌥Space anywhere to talk; the page opens itself and drives the mic.
        hotkey = Hotkey { [weak self] down in self?.deliver(["type": "hotkey", "down": down]) }
    }

    private func trackPointer() {
        let mouse = NSEvent.mouseLocation
        guard mouse != lastPointer else { return }
        lastPointer = mouse
        let frame = panel.frame
        // Page coordinates: origin top-left of the panel, y downward.
        deliver(["type": "pointer", "x": mouse.x - frame.minX, "y": frame.maxY - mouse.y])
    }

    /// ADR 0038: while operator works the panel's mark is Astrid's body
    /// beside the cursor tip. Puts the mark's centre on the overlay logo's
    /// centre (global Quartz points); the speech bubble sits to its right.
    func follow(logo: CGPoint, primaryHeight: CGFloat) {
        let point = NSPoint(x: logo.x, y: primaryHeight - logo.y)
        let screen = NSScreen.screens.first { $0.frame.contains(point) } ?? NSScreen.main
        guard let visible = screen?.visibleFrame else { return }
        let size = panel.frame.size
        var origin = NSPoint(x: point.x - Self.markCentre.x, y: point.y - size.height + Self.markCentre.y)
        origin.x = min(max(origin.x, visible.minX), visible.maxX - size.width)
        origin.y = min(max(origin.y, visible.minY), visible.maxY - size.height)
        if panel.frame.origin != origin { panel.setFrameOrigin(origin) }
    }

    /// The mark's centre from the panel's top-left in the operator layout.
    private static let markCentre = CGPoint(x: 64, y: 64)

    /// Lets one posted action reach the app under the panel.
    func passThrough(_ on: Bool) {
        panel.ignoresMouseEvents = on
    }

    /// The mark's centre in global Quartz points, where the cursor Astrid starts.
    private func markPoint() -> CGPoint {
        let frame = panel.frame
        let primaryHeight = NSScreen.screens.first?.frame.height ?? frame.maxY
        return CGPoint(x: frame.minX + 34, y: primaryHeight - frame.maxY + 34)
    }

    private func place() {
        let screen = NSScreen.screens.first { $0.frame.contains(anchor) } ?? NSScreen.main ?? NSScreen.screens.first
        guard let visible = screen?.visibleFrame else { return }
        let size = NSSize(width: min(contentSize.width, 520), height: min(contentSize.height, visible.height - 2 * Self.margin))
        var frame = NSRect(x: anchor.x - size.width / 2, y: anchor.y, width: size.width, height: size.height)
        frame.origin.x = min(max(frame.minX, visible.minX), visible.maxX - frame.width)
        frame.origin.y = min(max(frame.minY, visible.minY), visible.maxY - frame.height)
        panel.setFrame(frame, display: true)
    }

    // MARK: Web → native

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        guard let body = message.body as? [String: Any], let type = body["type"] as? String else { return }
        switch type {
        case "size":
            if let w = body["width"] as? Double, let h = body["height"] as? Double, w > 0, h > 0 {
                contentSize = NSSize(width: w, height: h)
                if following {
                    panel.setContentSize(NSSize(width: min(w, 520), height: h))
                } else {
                    place()
                }
            }
        case "drag-start":
            startDrag()
        case "drag":
            // Web y grows downward; screen y grows upward.
            if let dx = body["dx"] as? Double, let dy = body["dy"] as? Double, dx.isFinite, dy.isFinite {
                anchor.x += dx
                anchor.y -= dy
                place()
                let frame = panel.frame
                anchor = NSPoint(x: frame.midX, y: frame.minY)
            }
        case "expanded":
            expanded = body["expanded"] as? Bool == true
            if body["expanded"] as? Bool == true {
                speaker.prepare()
                NSApp.activate(ignoringOtherApps: true)
                panel.makeKey()
            } else {
                panel.resignKey()
            }
        case "listen":
            // Talking over Astrid interrupts her (barge-in).
            if canvas.isVisible { canvas.deliver(["type": "listening", "on": body["on"] as? Bool == true], near: anchor) }
            if body["on"] as? Bool == true {
                // One user of the microphone at a time: push-to-talk wins.
                if !listening { listening = true; wakeListener?.pause() }
                speaker.stop()
                speaker.prepare()
                voice.start()
            } else {
                voice.stop()
                if listening {
                    listening = false
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) { [weak self] in self?.wakeListener?.resume() }
                }
        speaker.shutdown()
            }
        case "speak":
            if let text = body["text"] as? String { speaker.say(String(text.prefix(4000))) }
        case "speak-stop":
            speaker.stop()
        case "ask", "approve", "cancel":
            if type == "ask" { asking = true }
            forward(body)
        case "nudge-choice":
            if let id = body["id"] as? String, let choice = body["choice"] as? String {
                nudges?.answer(id: id, choice: choice)
            }
        case "choice":
            // ADR 0038: a button on the operator box.
            if let id = body["id"] as? NSNumber, let choice = body["choice"] as? String {
                send(["type": "choice", "id": id, "choice": String(choice.prefix(16))])
            }
        case "utterance":
            // ADR 0038 voice: an answer to the operator card on screen.
            if let text = body["text"] as? String { send(["type": "utterance", "text": String(text.prefix(500))]) }
        case "open":
            // Gmail messages and (ADR 0041) Google Calendar days, in the browser.
            if let text = body["url"] as? String, let url = URL(string: text), url.scheme == "https",
               url.host == "mail.google.com" || GoogleCalendarLink.isAllowed(url) {
                NSWorkspace.shared.open(url)
            }
        default:
            break
        }
    }

    /// Moves the panel natively with the mouse until the button is released:
    /// no per-move round trip through the page, so the drag stays smooth.
    private func startDrag() {
        dragTimer?.invalidate()
        let offset = NSPoint(x: anchor.x - NSEvent.mouseLocation.x, y: anchor.y - NSEvent.mouseLocation.y)
        let timer = Timer(timeInterval: 1.0 / 120, repeats: true) { [weak self] timer in
            guard let self else { return timer.invalidate() }
            let mouse = NSEvent.mouseLocation
            let next = NSPoint(x: mouse.x + offset.x, y: mouse.y + offset.y)
            if next != self.anchor {
                self.anchor = next
                self.panel.setFrameOrigin(NSPoint(x: next.x - self.panel.frame.width / 2, y: next.y))
            }
            if NSEvent.pressedMouseButtons & 1 == 0 {
                timer.invalidate()
                self.dragTimer = nil
                self.place()
                let frame = self.panel.frame
                self.anchor = NSPoint(x: frame.midX, y: frame.minY)
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        dragTimer = timer
    }

    // MARK: Runtime child

    private func forward(_ message: [String: Any]) {
        if child?.isRunning != true { startChild() }
        guard let input = childInput, let data = try? JSONSerialization.data(withJSONObject: message) else { return }
        try? input.write(contentsOf: data + Data("\n".utf8))
    }

    /// Replies to the running child only; never starts one.
    func send(_ message: [String: Any]) {
        guard child?.isRunning == true, let input = childInput,
              let data = try? JSONSerialization.data(withJSONObject: message) else { return }
        childWrites.async { try? input.write(contentsOf: data + Data("\n".utf8)) }
    }

    private func startChild() {
        let process = Process()
        process.executableURL = config.astrid
        var arguments = ["presence-ask", "--model", config.model]
        if let operatorModel = config.operatorModel { arguments += ["--operator-model", operatorModel] }
        if let mail = config.mail { arguments += ["--mail-source", mail] }
        if config.calendar { arguments.append("--calendar") }
        process.arguments = arguments
        let input = Pipe(), output = Pipe()
        process.standardInput = input
        process.standardOutput = output
        process.standardError = FileHandle.nullDevice
        output.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let chunk = handle.availableData
            DispatchQueue.main.async { self?.receive(chunk) }
        }
        process.terminationHandler = { [weak self] _ in
            DispatchQueue.main.async {
                self?.deliver(["type": "error", "message": "Astrid's runtime stopped. Ask again to restart it."])
                self?.deliver(["type": "done", "outcome": "failed"])
            }
        }
        do {
            try process.run()
            child = process
            childInput = input.fileHandleForWriting
        } catch {
            deliver(["type": "error", "message": "Could not start astrid at \(config.astrid.path)"])
            deliver(["type": "done", "outcome": "failed"])
        }
    }

    private func receive(_ chunk: Data) {
        guard !chunk.isEmpty else { return }
        buffer.append(chunk)
        if buffer.count > 8 * 1024 * 1024 { buffer.removeAll() }
        while let newline = buffer.firstIndex(of: UInt8(ascii: "\n")) {
            let line = buffer[buffer.startIndex..<newline]
            buffer.removeSubrange(buffer.startIndex...newline)
            // Only well-formed JSON objects reach the page.
            guard let object = try? JSONSerialization.jsonObject(with: line) as? [String: Any] else { continue }
            route(object)
        }
    }

    /// Canvas events go to the canvas; answers go to both once it has drawn.
    private func route(_ object: [String: Any]) {
        let type = object["type"] as? String
        // ADR 0038: operator traffic goes to the presence app, not the page.
        switch type {
        case "present":
            if let event = object["event"] as? [String: Any] { host?.operatorPresent(event) }
            return
        case "capture":
            if let id = object["request_id"] as? String, let display = (object["display"] as? NSNumber)?.intValue {
                host?.operatorCapture(requestId: id, display: display) { [weak self] reply in self?.send(reply) }
            }
            return
        case "act":
            let id = object["request_id"] as? String ?? ""
            host?.operatorAct(object) { [weak self] outcome in
                self?.send(["type": "acted", "request_id": id, "outcome": outcome])
            }
            return
        case "calendar":
            // ADR 0041: only the ask child's calendar requests, answered by EventKit.
            guard config.calendar else {
                send(["type": "calendar_result", "request_id": object["request_id"] ?? "", "ok": false, "reason": "calendar_access_denied"])
                return
            }
            let service = calendar
            calendarQueue.async { [weak self] in
                let reply = service.handle(object)
                DispatchQueue.main.async { self?.send(reply) }
            }
            return
        case "operator":
            // The cursor tip takes over the pointing; the mark rides with it.
            if object["on"] as? Bool == true {
                following = true
                panel.resignKey()
                host?.operatorBegan(from: markPoint())
            } else {
                following = false
                panel.ignoresMouseEvents = false
                host?.operatorEnded()
                place()
            }
            deliver(object)
            return
        case "step":
            // The box shows the question; voice can answer it too.
            deliver(["type": "operator-card", "id": object["id"] ?? 0, "text": object["text"] ?? "",
                     "step": object["step"] ?? NSNull(), "total": object["total"] ?? NSNull(),
                     "buttons": (object["buttons"] as? [[String: Any]] ?? []).compactMap { b -> [String: String]? in
                         guard let id = b["id"] as? String, let label = b["label"] as? String else { return nil }
                         return ["id": id, "label": label]
                     }])
            return
        case "step_clear":
            deliver(["type": "operator-card-clear", "id": object["id"] ?? 0])
            return
        case "done":
            asking = false
            if following {
                following = false
                host?.operatorEnded()
                place()
            }
        default:
            break
        }
        if type == "canvas" {
            canvas.deliver(object, near: anchor)
            return
        }
        // A follow-up asked while the canvas is up shows its state there too.
        if ["state", "done"].contains(type ?? ""), canvas.isVisible { canvas.deliver(object, near: anchor) }
        deliver(object)
    }

    // MARK: Nudges (ADR 0042)

    /// A short bubble beside the mark; spoken only when the rule said so.
    func showNudge(_ nudge: Nudge, spoken: Bool) {
        deliver(["type": "nudge", "id": nudge.id, "text": nudge.text, "speak": spoken,
                 "buttons": nudge.buttons.map { ["id": $0.id, "label": $0.label] }])
    }

    func clearNudge(id: String) {
        deliver(["type": "nudge-clear", "id": id])
    }

    /// A nudge's yes: the page asks this exactly as if it had been typed.
    func askFromNudge(_ text: String) {
        deliver(["type": "nudge-ask", "text": String(text.prefix(500))])
    }

    var astridURL: URL { config.astrid }

    private func deliver(_ object: Any) {
        guard let data = try? JSONSerialization.data(withJSONObject: object), let json = String(data: data, encoding: .utf8) else { return }
        web.evaluateJavaScript("window.astrid && window.astrid.receive(\(json))")
    }

    /// Debug: with ASTRID_ASK_SNAPSHOT=/path.png, save the panel's own web
    /// content (never the screen) once the page has loaded.
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        // Debug: ASTRID_SAY="text" speaks once the page is up, to check playback.
        if let text = ProcessInfo.processInfo.environment["ASTRID_SAY"] {
            speaker.prepare()
            DispatchQueue.main.asyncAfter(deadline: .now() + 10) { [weak self] in self?.speaker.say(text) }
        }
        guard let path = ProcessInfo.processInfo.environment["ASTRID_ASK_SNAPSHOT"] else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
            webView.takeSnapshot(with: nil) { image, _ in
                guard let image, let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff),
                      let png = rep.representation(using: .png, properties: [:]) else { return }
                try? png.write(to: URL(fileURLWithPath: path))
            }
        }
    }

    func stop() {
        pointerTimer?.invalidate()
        wakeListener?.stop()
        voice.stop()
        child?.terminate()
    }
}
