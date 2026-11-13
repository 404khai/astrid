import AppKit
import WebKit

/// ADR 0036: Astrid's live canvas. A floating panel that shows the web canvas
/// page and receives the runtime's already-validated canvas events through the
/// ask controller. It takes clicks (close) but never keyboard focus, and it
/// only appears once a drawing begins.
final class CanvasController: NSObject, WKScriptMessageHandler {
    private let panel: NSPanel
    private let web: WKWebView
    private var loaded = false
    private var queued: [String] = []

    init(page: URL) {
        panel = NSPanel(contentRect: NSRect(x: 0, y: 0, width: 960, height: 680), styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        let configuration = WKWebViewConfiguration()
        configuration.setURLSchemeHandler(UIFiles(root: page.deletingLastPathComponent()), forURLScheme: "astrid-ui")
        web = WKWebView(frame: .zero, configuration: configuration)
        super.init()
        configuration.userContentController.add(self, name: "astrid")
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.becomesKeyOnlyIfNeeded = true
        panel.isMovableByWindowBackground = true
        web.setValue(false, forKey: "drawsBackground")
        panel.contentView = web
        web.load(URLRequest(url: URL(string: "astrid-ui://app/\(page.lastPathComponent)")!))
    }

    var isVisible: Bool { panel.isVisible }

    /// Shows the canvas centred on the screen holding `near`, leaving room
    /// below for the ask panel.
    private func show(near: NSPoint) {
        guard !panel.isVisible else { return }
        let screen = NSScreen.screens.first { $0.frame.contains(near) } ?? NSScreen.main ?? NSScreen.screens.first
        guard let visible = screen?.visibleFrame else { return }
        let width = min(960, visible.width - 40)
        let height = min(width * 0.71, visible.height - 140)
        let frame = NSRect(x: visible.midX - width / 2, y: visible.maxY - height - 24, width: width, height: height)
        panel.setFrame(frame, display: true)
        panel.orderFrontRegardless()
    }

    /// Relays one runtime message to the page; a drawing's start shows the panel.
    func deliver(_ object: [String: Any], near: NSPoint) {
        if object["type"] as? String == "canvas", object["event"] as? String == "begin" { show(near: near) }
        guard let data = try? JSONSerialization.data(withJSONObject: object), let json = String(data: data, encoding: .utf8) else { return }
        let script = "window.astrid && window.astrid.receive(\(json))"
        if loaded { web.evaluateJavaScript(script) } else { queued.append(script) }
    }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        guard let body = message.body as? [String: Any], let type = body["type"] as? String else { return }
        switch type {
        case "ready":
            loaded = true
            queued.forEach { web.evaluateJavaScript($0) }
            queued.removeAll()
        case "canvas-close":
            panel.orderOut(nil)
        default:
            break
        }
    }
}
