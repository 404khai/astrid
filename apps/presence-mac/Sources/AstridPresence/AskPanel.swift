import AppKit
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
}

final class AskController: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
    private let config: AskConfig
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
    private var hotkey: Hotkey?
    private var dragTimer: Timer?
    private let speaker: Speaker
    /// ADR 0036: the live canvas, created on the first drawing.
    private lazy var canvas = CanvasController(page: config.ui.deletingLastPathComponent().appendingPathComponent("canvas.html"))

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
        speaker.onSpeaking = { [weak self] on in self?.deliver(["type": "speaking", "on": on]) }
        voice.onTranscript = { [weak self] text, final in self?.deliver(["type": "transcript", "text": text, "final": final]) }
        voice.onError = { [weak self] message in self?.deliver(["type": "voice-error", "message": message]) }
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

    /// Places the panel around its anchor, kept fully on the anchor's screen.
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
                place()
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
                speaker.stop()
                speaker.prepare()
                voice.start()
            } else {
                voice.stop()
        speaker.shutdown()
            }
        case "speak":
            if let text = body["text"] as? String { speaker.say(String(text.prefix(4000))) }
        case "speak-stop":
            speaker.stop()
        case "ask", "approve", "cancel":
            forward(body)
        case "open":
            // Only Gmail message links, opened in the user's browser.
            if let text = body["url"] as? String, let url = URL(string: text), url.scheme == "https", url.host == "mail.google.com" {
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

    private func startChild() {
        let process = Process()
        process.executableURL = config.astrid
        var arguments = ["presence-ask", "--model", config.model]
        if let mail = config.mail { arguments += ["--mail-source", mail] }
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
        if type == "canvas" {
            canvas.deliver(object, near: anchor)
            return
        }
        // A follow-up asked while the canvas is up shows its state there too.
        if ["state", "done"].contains(type ?? ""), canvas.isVisible { canvas.deliver(object, near: anchor) }
        deliver(object)
    }

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
        voice.stop()
        child?.terminate()
    }
}

/// Read-only file server for the ask UI folder; never leaves that folder.
final class UIFiles: NSObject, WKURLSchemeHandler {
    private let root: URL
    init(root: URL) { self.root = root.standardizedFileURL }

    func webView(_ webView: WKWebView, start task: WKURLSchemeTask) {
        guard let url = task.request.url else { return }
        let file = root.appendingPathComponent(url.path).standardizedFileURL
        guard file.path.hasPrefix(root.path + "/"), let data = try? Data(contentsOf: file) else {
            task.didFailWithError(URLError(.fileDoesNotExist))
            return
        }
        let types = ["html": "text/html", "js": "text/javascript", "css": "text/css", "woff2": "font/woff2", "woff": "font/woff", "svg": "image/svg+xml", "png": "image/png", "json": "application/json"]
        let mime = types[file.pathExtension.lowercased()] ?? "application/octet-stream"
        task.didReceive(HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1", headerFields: ["Content-Type": mime, "Content-Length": String(data.count)])!)
        task.didReceive(data)
        task.didFinish()
    }

    func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {}
}
