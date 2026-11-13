import AstridWebUI
import DesktopCore
import SwiftUI
import WebKit

/// One reply's drawing, rendered by the same canvas page the presence app uses
/// (`web/src/canvas`, ADR 0036) in embed mode. A reply made in this window
/// streams in shape by shape; a saved one is replayed at once. Dragging turns a
/// 3D figure and pinch zooms; plain scrolling stays with the thread.
struct DiagramView: NSViewRepresentable {
    var ui: URL
    var shapes: [JSONValue]
    var animate: Bool
    var finished: Bool

    func makeCoordinator() -> Coordinator { Coordinator(animate: animate) }

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.setURLSchemeHandler(UIFiles(root: ui), forURLScheme: "astrid-ui")
        configuration.userContentController.add(WeakHandler(context.coordinator), name: "astrid")
        let web = WKWebView(frame: .zero, configuration: configuration)
        web.setValue(false, forKey: "drawsBackground")
        web.setAccessibilityLabel(shapes.first { $0["shape"]?.string == "title" }?["text"]?.string ?? "Astrid's drawing")
        context.coordinator.web = web
        context.coordinator.update(shapes: shapes, finished: finished)
        web.load(URLRequest(url: URL(string: "astrid-ui://app/canvas.html?embed")!))
        return web
    }

    func updateNSView(_ web: WKWebView, context: Context) {
        context.coordinator.update(shapes: shapes, finished: finished)
    }

    static func dismantleNSView(_ web: WKWebView, coordinator: Coordinator) {
        web.configuration.userContentController.removeScriptMessageHandler(forName: "astrid")
    }

    final class Coordinator: NSObject, WKScriptMessageHandler {
        /// Fixed when the view is made, so a finished live reply isn't replayed.
        let animate: Bool
        weak var web: WKWebView?
        private var ready = false
        private var shapes: [JSONValue] = []
        private var finished = false
        private var sent = 0
        private var begun = false
        private var ended = false

        init(animate: Bool) { self.animate = animate }

        func update(shapes: [JSONValue], finished: Bool) {
            self.shapes = shapes
            self.finished = finished
            sync()
        }

        func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
            guard let body = message.body as? [String: Any], body["type"] as? String == "ready" else { return }
            ready = true
            sync()
        }

        private func sync() {
            guard ready else { return }
            if !animate {
                guard shapes.count != sent else { return }
                post(["type": "canvas", "event": "replay", "shapes": shapes.map(\.any)])
                sent = shapes.count
                return
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

/// WKUserContentController keeps its handlers alive; this breaks the cycle.
private final class WeakHandler: NSObject, WKScriptMessageHandler {
    weak var target: WKScriptMessageHandler?
    init(_ target: WKScriptMessageHandler) { self.target = target }
    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        target?.userContentController(controller, didReceive: message)
    }
}
