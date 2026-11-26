import AstridWebUI
import DesktopCore
import SwiftUI
import WebKit

/// One reply's drawing, rendered by the same canvas page the presence app uses
/// (`web/src/canvas`, ADR 0036) in embed mode. A reply made in this window
/// streams in shape by shape; a saved one is replayed at once. Dragging turns a
/// 3D figure and pinch zooms; plain scrolling stays with the thread.
///
/// Astrid Weaver: tapping a part asks about it (`onAsk`), the toolbar's export
/// saves a PNG or SVG through a save panel, and an edit-only reply shows the
/// drawing it changes at once (`prefix`) and streams just the edits.
struct DiagramView: NSViewRepresentable {
    var ui: URL
    var shapes: [JSONValue]
    var animate: Bool
    var finished: Bool
    /// False for a thumbnail: clicks fall through to whatever is behind it.
    var interactive = true
    var prefix = 0
    /// Names exported files.
    var title: String?
    var onAsk: ((String) -> Void)?

    init(ui: URL, shapes: [JSONValue], animate: Bool, finished: Bool, interactive: Bool = true,
         prefix: Int = 0, title: String? = nil, onAsk: ((String) -> Void)? = nil) {
        self.ui = ui
        self.shapes = shapes
        self.animate = animate
        self.finished = finished
        self.interactive = interactive
        self.prefix = prefix
        self.title = title
        self.onAsk = onAsk
    }

    func makeCoordinator() -> Coordinator { Coordinator(animate: animate) }

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.setURLSchemeHandler(UIFiles(root: ui), forURLScheme: "astrid-ui")
        configuration.userContentController.add(WeakHandler(context.coordinator), name: "astrid")
        let web = PassthroughWebView(frame: .zero, configuration: configuration)
        web.interactive = interactive
        web.setValue(false, forKey: "drawsBackground")
        web.setAccessibilityLabel(shapes.first { $0["shape"]?.string == "title" }?["text"]?.string ?? "Astrid's drawing")
        context.coordinator.web = web
        context.coordinator.onAsk = onAsk
        context.coordinator.title = title
        context.coordinator.update(shapes: shapes, finished: finished, prefix: prefix)
        web.load(URLRequest(url: URL(string: "astrid-ui://app/canvas.html?embed\(interactive ? "" : "&thumb")")!))
        return web
    }

    func updateNSView(_ web: WKWebView, context: Context) {
        context.coordinator.onAsk = onAsk
        context.coordinator.title = title
        context.coordinator.update(shapes: shapes, finished: finished, prefix: prefix)
    }

    static func dismantleNSView(_ web: WKWebView, coordinator: Coordinator) {
        web.configuration.userContentController.removeScriptMessageHandler(forName: "astrid")
    }

    final class Coordinator: NSObject, WKScriptMessageHandler {
        /// Fixed when the view is made, so a finished live reply isn't replayed.
        let animate: Bool
        weak var web: WKWebView?
        var onAsk: ((String) -> Void)?
        var title: String?
        private var ready = false
        private var prefix = 0
        private var shapes: [JSONValue] = []
        private var finished = false
        private var sent = 0
        private var begun = false
        private var ended = false

        init(animate: Bool) { self.animate = animate }

        func update(shapes: [JSONValue], finished: Bool, prefix: Int = 0) {
            self.shapes = shapes
            self.finished = finished
            self.prefix = prefix
            sync()
        }

        func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
            guard let body = message.body as? [String: Any], let type = body["type"] as? String else { return }
            switch type {
            case "ready":
                ready = true
                sync()
            case "ask":
                // Plain text the page built from a validated part name; it goes
                // through the same ask path as anything typed.
                if let text = body["text"] as? String, !text.isEmpty { onAsk?(String(text.prefix(300))) }
            case "export":
                guard let format = (body["format"] as? String).flatMap(CanvasExport.Format.init(rawValue:)),
                      let payload = body["data"] as? String,
                      let data = CanvasExport.data(format: format, payload: payload) else { return }
                save(data, name: CanvasExport.fileName(body["name"] as? String ?? title ?? "", format: format), format: format)
            default:
                break
            }
        }

        /// Asks where to save an exported drawing, then writes it there.
        private func save(_ data: Data, name: String, format: CanvasExport.Format) {
            let panel = NSSavePanel()
            panel.nameFieldStringValue = name
            panel.allowedContentTypes = [format == .png ? .png : .svg]
            panel.canCreateDirectories = true
            let done: (NSApplication.ModalResponse) -> Void = { response in
                guard response == .OK, let url = panel.url else { return }
                do {
                    try data.write(to: url, options: .atomic)
                } catch {
                    NSAlert(error: error).runModal()
                }
            }
            if let window = web?.window { panel.beginSheetModal(for: window, completionHandler: done) } else { done(panel.runModal()) }
        }

        private func sync() {
            guard ready else { return }
            if !animate {
                guard shapes.count != sent else { return }
                post(["type": "canvas", "event": "replay", "shapes": shapes.map(\.any)])
                sent = shapes.count
                return
            }
            // An edit-only reply: show the drawing it changes at once, then
            // stream the edits onto it.
            if prefix > 0, sent < prefix, shapes.count >= prefix {
                post(["type": "canvas", "event": "replay", "shapes": shapes[..<prefix].map(\.any)])
                sent = prefix
                begun = false
            }
            if !begun {
                post(["type": "canvas", "event": "begin", "width": 1000, "height": 640])
                begun = true
            }
            while sent < shapes.count {
                post(["type": "canvas", "event": "shape", "shape": shapes[sent].any])
                sent += 1
            }
            if finished, !ended {
                post(["type": "canvas", "event": "end"])
                ended = true
            }
        }

        private func post(_ object: [String: Any]) {
            guard let data = try? JSONSerialization.data(withJSONObject: object), let json = String(data: data, encoding: .utf8) else { return }
            web?.evaluateJavaScript("window.astrid && window.astrid.receive(\(json))")
        }
    }
}

/// A web view that can step out of hit testing, so a thumbnail's clicks reach
/// the SwiftUI button around it.
final class PassthroughWebView: WKWebView {
    var interactive = true
    override func hitTest(_ point: NSPoint) -> NSView? {
        interactive ? super.hitTest(point) : nil
    }
}

/// WKUserContentController keeps its handlers alive; this breaks the cycle.
final class WeakHandler: NSObject, WKScriptMessageHandler {
    weak var target: WKScriptMessageHandler?
    init(_ target: WKScriptMessageHandler) { self.target = target }
    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        target?.userContentController(controller, didReceive: message)
    }
}
