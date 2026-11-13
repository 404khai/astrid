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
}

final class AskController: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
    private let config: AskConfig
    private let panel: AskPanel
    private let web: WKWebView
    private var child: Process?
    private var childInput: FileHandle?
    private var buffer = Data()
    private var contentSize = NSSize(width: 84, height: 76)
    private static let margin: CGFloat = 20

    init(config: AskConfig) {
        self.config = config
        panel = AskPanel(contentRect: NSRect(x: 0, y: 0, width: 84, height: 76), styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        let configuration = WKWebViewConfiguration()
        // Serve the built UI from its folder over a private scheme so ES module
        // scripts load with a real origin (file:// gives them a null origin).
        configuration.setURLSchemeHandler(UIFiles(root: config.ui.deletingLastPathComponent()), forURLScheme: "astrid-ui")
        web = WKWebView(frame: .zero, configuration: configuration)
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
    }

    /// Anchored to the bottom-right of the main screen's visible area.
    private func place() {
        guard let screen = NSScreen.main ?? NSScreen.screens.first else { return }
        let visible = screen.visibleFrame
        let size = NSSize(width: min(contentSize.width, 520), height: min(contentSize.height, visible.height - 2 * Self.margin))
        panel.setFrame(NSRect(x: visible.maxX - size.width - Self.margin, y: visible.minY + Self.margin, width: size.width, height: size.height), display: true)
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
        case "expanded":
            if body["expanded"] as? Bool == true {
                NSApp.activate(ignoringOtherApps: true)
                panel.makeKey()
            } else {
                panel.resignKey()
            }
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
            guard let object = try? JSONSerialization.jsonObject(with: line), object is [String: Any] else { continue }
            deliver(object)
        }
    }

    private func deliver(_ object: Any) {
        guard let data = try? JSONSerialization.data(withJSONObject: object), let json = String(data: data, encoding: .utf8) else { return }
        web.evaluateJavaScript("window.astrid && window.astrid.receive(\(json))")
    }

    /// Debug: with ASTRID_ASK_SNAPSHOT=/path.png, save the panel's own web
    /// content (never the screen) once the page has loaded.
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
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
