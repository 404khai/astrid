import AstridWebUI
import SwiftUI
import WebKit

/// Presence's voice ring round Astrid's mark (`web/voice.html`), fed the mic's
/// frames while she listens and her voice's while she speaks, as presence
/// feeds its own (ADR 0034). Tapping her interrupts.
struct VoiceRingView: NSViewRepresentable {
    var ui: URL
    @ObservedObject var voice: VoiceMode

    func makeCoordinator() -> Coordinator { Coordinator(voice: voice) }

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.setURLSchemeHandler(UIFiles(root: ui), forURLScheme: "astrid-ui")
        configuration.userContentController.add(WeakHandler(context.coordinator), name: "astrid")
        let web = WKWebView(frame: .zero, configuration: configuration)
        web.setValue(false, forKey: "drawsBackground")
        web.setAccessibilityLabel("Astrid")
        context.coordinator.web = web
        web.load(URLRequest(url: URL(string: "astrid-ui://app/voice.html")!))
        return web
    }

    func updateNSView(_ web: WKWebView, context: Context) {
        context.coordinator.sync()
    }

    static func dismantleNSView(_ web: WKWebView, coordinator: Coordinator) {
        coordinator.voice.onFrame = nil
        web.configuration.userContentController.removeScriptMessageHandler(forName: "astrid")
    }

    final class Coordinator: NSObject, WKScriptMessageHandler {
        let voice: VoiceMode
        weak var web: WKWebView?
        private var ready = false
        private var sent: String?

        init(voice: VoiceMode) {
            self.voice = voice
            super.init()
            voice.onFrame = { [weak self] frame in
                guard let self, self.ready else { return }
                self.post(frame.message)
            }
        }

        func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
            guard let body = message.body as? [String: Any] else { return }
            switch body["type"] as? String {
            case "ready":
                ready = true
                post(["type": "reduced", "on": NSWorkspace.shared.accessibilityDisplayShouldReduceMotion])
                sent = nil
                sync()
            case "interrupt":
                voice.interrupt()
            default:
                break
            }
        }

        /// The ring's state from voice mode's phase.
        func sync() {
            guard ready else { return }
            let state: String
            switch voice.phase {
            case .listening: state = voice.muted ? "idle" : "listening"
            case .thinking: state = "processing"
            case .speaking: state = "speaking"
            case .off: state = "idle"
            }
            guard state != sent else { return }
            sent = state
            post(["type": "state", "state": state])
        }

        private func post(_ object: [String: Any]) {
            guard let data = try? JSONSerialization.data(withJSONObject: object), let json = String(data: data, encoding: .utf8) else { return }
            web?.evaluateJavaScript("window.astrid && window.astrid.receive(\(json))")
        }
    }
}
