import AppKit
import AstridWebUI
import SwiftUI
import WebKit

/// A bot exported from Avatar Lab (avatars.bible-strong.app) as `.avatar.json`.
struct Blob: Identifiable, Equatable {
    var id: String
    var name: String
    var character: Any

    static func == (a: Blob, b: Blob) -> Bool { a.id == b.id && a.name == b.name }
}

/// Blobs: Avatar Lab's ten built-in bots, kept in the repo's `blobs/`
/// (AGPL-3.0, see its README), and any the user exports into Astrid's Blobs
/// folder. Up to five show in the sidebar, chosen in Settings.
enum Blobs {
    static let limit = 5
    /// Shown until the user picks: a round, a cloud, a cone, a cube, a pink one.
    static let defaults = ["06-sunee.avatar.json", "08-cloudee.avatar.json", "03-citrus.avatar.json", "02-freddy.avatar.json", "07-kirby.avatar.json"]
    static let maxBytes = 512 * 1024
    static let changed = Notification.Name("astrid.blobs.changed")

    static var folder: URL {
        if let path = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_BLOBS"] { return URL(fileURLWithPath: path, isDirectory: true) }
        return FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Astrid/Blobs", isDirectory: true)
    }

    /// Settings' Blobs icon: Giovanni, from the repo.
    static let icon: NSImage? = repoFile("giovanni.svg").flatMap { NSImage(contentsOf: $0) }

    /// The built-in bots, from the repo.
    static var builtIn: URL? { repoFile("blobs") }

    /// Every bot: the built-in ones, then the user's.
    static func all() -> [Blob] {
        (builtIn.map { load(from: $0, limit: 32) } ?? []) + load(from: folder, limit: 32)
    }

    /// The bots in the sidebar: the chosen ones, in the order chosen.
    static func load(shown: [String] = defaults) -> [Blob] {
        let every = all()
        let picked = shown.compactMap { id in every.first { $0.id == id } }
        return Array((picked.isEmpty ? every : picked).prefix(limit))
    }

    static func load(from folder: URL, limit: Int) -> [Blob] {
        let files = (try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: [.fileSizeKey])) ?? []
        return files
            .filter { $0.pathExtension == "json" }
            .sorted { $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending }
            .compactMap { url -> Blob? in
                guard let size = try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize, size <= maxBytes,
                      let data = try? Data(contentsOf: url),
                      let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
                let stem = url.lastPathComponent.replacingOccurrences(of: ".avatar.json", with: "").replacingOccurrences(of: ".json", with: "")
                let name = (object["name"] as? String).flatMap { $0.isEmpty ? nil : String($0.prefix(40)) } ?? stem.capitalized
                return Blob(id: url.lastPathComponent, name: name, character: object)
            }
            .prefix(limit)
            .map { $0 }
    }
}

/// The row of blobs in the sidebar, drawn by `web/blobs.html`.
struct BlobsRow: View {
    var ui: URL
    @ObservedObject var settings: AppSettings
    @ObservedObject var model: ChatModel
    @State private var blobs: [Blob]

    init(ui: URL, settings: AppSettings, model: ChatModel) {
        self.ui = ui
        self.settings = settings
        self.model = model
        _blobs = State(initialValue: Blobs.load(shown: settings.blobsShown))
    }

    var body: some View {
        Group {
            if settings.showBlobs, !blobs.isEmpty {
                BlobsWeb(ui: ui, blobs: blobs, selected: settings.blob ?? blobs.first?.id, busy: model.live != nil) { settings.blob = $0 }
                    .frame(minWidth: 0, maxWidth: .infinity, minHeight: 50, maxHeight: 50)
                    .clipped()
            }
        }
        .onChange(of: settings.blobsShown) { _, shown in blobs = Blobs.load(shown: shown) }
        .onReceive(NotificationCenter.default.publisher(for: Blobs.changed)) { _ in blobs = Blobs.load(shown: settings.blobsShown) }
    }
}

private struct BlobsWeb: NSViewRepresentable {
    var ui: URL
    var blobs: [Blob]
    var selected: String?
    var busy: Bool
    var pick: (String) -> Void

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.setURLSchemeHandler(UIFiles(root: ui), forURLScheme: "astrid-ui")
        configuration.userContentController.add(WeakHandler(context.coordinator), name: "astrid")
        let web = WKWebView(frame: .zero, configuration: configuration)
        web.setValue(false, forKey: "drawsBackground")
        context.coordinator.web = web
        context.coordinator.update(self)
        web.load(URLRequest(url: URL(string: "astrid-ui://app/blobs.html")!))
        return web
    }

    func updateNSView(_ web: WKWebView, context: Context) { context.coordinator.update(self) }

    static func dismantleNSView(_ web: WKWebView, coordinator: Coordinator) {
        web.configuration.userContentController.removeScriptMessageHandler(forName: "astrid")
    }

    final class Coordinator: NSObject, WKScriptMessageHandler {
        weak var web: WKWebView?
        private var ready = false
        private var view: BlobsWeb?
        private var sentBlobs: [Blob] = []
        private var sentBusy: Bool?

        func update(_ view: BlobsWeb) {
            self.view = view
            guard ready else { return }
            if view.blobs != sentBlobs {
                sentBlobs = view.blobs
                post(["type": "blobs", "items": view.blobs.map { ["id": $0.id, "name": $0.name, "character": $0.character] }, "selected": view.selected as Any])
            }
            if view.busy != sentBusy {
                sentBusy = view.busy
                post(["type": "state", "state": view.busy ? "thinking" : "idle"])
            }
        }

        func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
            guard let body = message.body as? [String: Any] else { return }
            switch body["type"] as? String {
            case "ready":
                ready = true
                sentBlobs = []
                sentBusy = nil
                if let view { update(view) }
            case "select":
                if let id = body["id"] as? String { view?.pick(id) }
            default: break
            }
        }

        private func post(_ object: [String: Any]) {
            guard let data = try? JSONSerialization.data(withJSONObject: object), let json = String(data: data, encoding: .utf8) else { return }
            web?.evaluateJavaScript("window.astrid && window.astrid.receive(\(json))")
        }
    }
}
