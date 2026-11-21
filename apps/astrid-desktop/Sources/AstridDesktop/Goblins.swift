import AppKit
import AstridWebUI
import SwiftUI
import WebKit

/// A goblin: a mark from the Goblins studio (docs › Goblins, from
/// github.com/404khai/goblins v2, MIT), an `astrid/mark@1` object (form,
/// shape, corners, arch colours, eyes, mood) as the page draws it.
struct Goblin: Identifiable, Equatable {
    var id: String
    var name: String
    var character: Any

    static func == (a: Goblin, b: Goblin) -> Bool { a.id == b.id && a.name == b.name }
}

/// Goblins: the built-in clan, kept as astrid/mark@1 files in the repo's
/// `goblins/` (written by web/scripts/build-goblin-clan.ts), and any the user
/// pastes from the Goblins studio into Astrid's Goblins folder. A file may
/// hold several goblins. Up to five show in the sidebar, chosen in Settings.
enum Goblins {
    static let limit = 5
    /// Shown until the user picks: Bramble, Grub, Pip, Tansy and Hob.
    static let defaults = ["03-bramble.goblin.json#0", "01-grub.goblin.json#0", "04-pip.goblin.json#0", "07-tansy.goblin.json#0", "12-hob.goblin.json#0"]
    static let maxBytes = 512 * 1024
    static let changed = Notification.Name("astrid.goblins.changed")

    static var folder: URL {
        if let path = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_GOBLINS"] { return URL(fileURLWithPath: path, isDirectory: true) }
        return FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Astrid/Goblins", isDirectory: true)
    }

    /// The built-in bots, from the repo.
    static var builtIn: URL? { repoFile("goblins") }

    /// Every bot: the built-in ones, then the user's.
    static func all() -> [Goblin] {
        (builtIn.map { load(from: $0, limit: 32) } ?? []) + load(from: folder, limit: 32)
    }

    /// The bots in the sidebar: the chosen ones, in the order chosen.
    static func load(shown: [String] = defaults) -> [Goblin] {
        let every = all()
        let picked = shown.compactMap { id in every.first { $0.id == id } }
        return Array((picked.isEmpty ? every : picked).prefix(limit))
    }

    static func load(from folder: URL, limit: Int) -> [Goblin] {
        let files = (try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: [.fileSizeKey])) ?? []
        return files
            .filter { $0.pathExtension == "json" }
            .sorted { $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending }
            .flatMap { url -> [Goblin] in
                guard let size = try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize, size <= maxBytes,
                      let data = try? Data(contentsOf: url),
                      let json = try? JSONSerialization.jsonObject(with: data) else { return [] }
                return recipes(in: json).enumerated().map { index, recipe in
                    let name = (recipe["name"] as? String).flatMap { $0.isEmpty ? nil : String($0.prefix(40)) } ?? "AstroNaut"
                    return Goblin(id: "\(url.lastPathComponent)#\(index)", name: name, character: recipe)
                }
            }
            .prefix(limit)
            .map { $0 }
    }
}

struct GoblinAddError: Error { var message: String }

extension Goblins {
    /// Adds pasted goblin JSON (marks from the studio's Copy JSON): checked like
    /// a file, then saved as one file in the Goblins folder (0600). Returns the
    /// names added.
    static func add(json text: String) -> Result<[String], GoblinAddError> {
        let data = Data(text.trimmingCharacters(in: .whitespacesAndNewlines).utf8)
        guard data.count <= maxBytes else { return .failure(.init(message: "That's too big for an AstroNaut (512 KB at most).")) }
        guard let json = try? JSONSerialization.jsonObject(with: data) else {
            return .failure(.init(message: "That isn't valid JSON."))
        }
        let found = recipes(in: json)
        guard !found.isEmpty else { return .failure(.init(message: "No AstroNauts in that: each needs arch colours (\"top\") and eyes.")) }
        let file: [String: Any] = ["$schema": "astrid/marks@1", "marks": found]
        guard let out = try? JSONSerialization.data(withJSONObject: file, options: [.prettyPrinted, .sortedKeys]) else {
            return .failure(.init(message: "Couldn't save those AstroNauts."))
        }
        let names = found.map { ($0["name"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? "AstroNaut" }
        let slug = names[0].lowercased().filter { $0.isLetter || $0.isNumber }
        let url = folder.appendingPathComponent("\(slug.isEmpty ? "goblin" : slug)-\(Int(Date().timeIntervalSince1970)).goblin.json")
        do {
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            try out.write(to: url, options: .atomic)
            try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
        } catch {
            return .failure(.init(message: "Couldn't save those AstroNauts: \(error.localizedDescription)"))
        }
        return .success(names)
    }
}

/// The goblins in pasted or stored JSON: one `astrid/mark@1` mark, a list of
/// them, or `{"marks": [...]}`. A mark needs its arch colours (`top`, or the
/// studio's older `arch`) and `eyes`; anything else, such as an old 3D goblin
/// recipe, is left out. The page fills in what is missing.
func recipes(in json: Any) -> [[String: Any]] {
    let list: [Any]
    if let array = json as? [Any] { list = array }
    else if let object = json as? [String: Any], let marks = object["marks"] as? [Any] { list = marks }
    else { list = [json] }
    func colours(_ value: Any?) -> Bool { value is String || (value as? [Any])?.allSatisfy { $0 is String } == true }
    return list.compactMap { $0 as? [String: Any] }
        .filter { colours($0["top"] ?? $0["arch"]) && colours($0["eyes"]) }
        .prefix(32).map { $0 }
}

/// The row of goblins in the sidebar, drawn by `web/goblins.html`.
struct GoblinsRow: View {
    var ui: URL
    @ObservedObject var settings: AppSettings
    @ObservedObject var model: ChatModel
    @State private var goblins: [Goblin]

    init(ui: URL, settings: AppSettings, model: ChatModel) {
        self.ui = ui
        self.settings = settings
        self.model = model
        _goblins = State(initialValue: Goblins.load(shown: settings.goblinsShown))
    }

    var body: some View {
        Group {
            if settings.showGoblins, !goblins.isEmpty {
                GoblinsWeb(ui: ui, goblins: goblins, selected: settings.goblin ?? goblins.first?.id, busy: model.live != nil) { settings.goblin = $0 }
                    .frame(minWidth: 0, maxWidth: .infinity, minHeight: 50, maxHeight: 50)
                    .clipped()
            }
        }
        .onChange(of: settings.goblinsShown) { _, shown in goblins = Goblins.load(shown: shown) }
        .onReceive(NotificationCenter.default.publisher(for: Goblins.changed)) { _ in goblins = Goblins.load(shown: settings.goblinsShown) }
    }
}

private struct GoblinsWeb: NSViewRepresentable {
    var ui: URL
    var goblins: [Goblin]
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
        web.load(URLRequest(url: URL(string: "astrid-ui://app/goblins.html")!))
        return web
    }

    func updateNSView(_ web: WKWebView, context: Context) { context.coordinator.update(self) }

    static func dismantleNSView(_ web: WKWebView, coordinator: Coordinator) {
        web.configuration.userContentController.removeScriptMessageHandler(forName: "astrid")
    }

    final class Coordinator: NSObject, WKScriptMessageHandler {
        weak var web: WKWebView?
        private var ready = false
        private var view: GoblinsWeb?
        private var sentGoblins: [Goblin] = []
        private var sentBusy: Bool?

        func update(_ view: GoblinsWeb) {
            self.view = view
            guard ready else { return }
            if view.goblins != sentGoblins {
                sentGoblins = view.goblins
                post(["type": "goblins", "items": view.goblins.map { ["id": $0.id, "name": $0.name, "character": $0.character] }, "selected": view.selected as Any])
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
                sentGoblins = []
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
