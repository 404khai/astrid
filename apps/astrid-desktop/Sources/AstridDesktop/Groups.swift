import AppKit
import AstridWebUI
import DesktopCore
import SwiftUI
import WebKit

/// Goblin groups (cosmetic for now, ADR 0037 amendment): pick goblins with the
/// Giovanni button in the composer, make a group, and it shows in the sidebar
/// as its goblins in a dark circle with its name, opening a mock group chat.
/// Everything is drawn by `web/group.html`; no model is involved.

/// Bramble as a symbol-style glyph, like an SF Symbol: the maintainer's
/// beardless recipe (wide head, long ears, bulb nose, beady eyes, gnome hat)
/// drawn by the goblins library's own glyph renderer into
/// `goblins/bramble-glyph.svg` (web/scripts/build-bramble-glyph.ts), shown as a
/// template image so it takes the button's colour.
struct BrambleGlyph: View {
    var size: CGFloat
    var color: Color = .white

    static let image: NSImage? = {
        guard let url = repoFile("goblins/bramble-glyph.svg"), let image = NSImage(contentsOf: url) else { return nil }
        image.isTemplate = true
        return image
    }()

    var body: some View {
        Group {
            if let image = Self.image {
                Image(nsImage: image).resizable().interpolation(.high).scaledToFit()
            } else {
                Image(systemName: "face.smiling").resizable().scaledToFit()
            }
        }
        .foregroundStyle(color)
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }
}

/// The composer's goblin button, beside the waveform: opens the goblin picker.
struct GoblinButton: View {
    @ObservedObject var model: ChatModel
    @EnvironmentObject private var settings: AppSettings
    @State private var open = false
    @Environment(\.palette) private var palette

    var body: some View {
        Button { open.toggle() } label: {
            BrambleGlyph(size: 21).frame(width: 22, height: 22)
        }
        .buttonStyle(.glass)
        .buttonBorderShape(.circle)
        .controlSize(.large)
        .help("Goblins")
        .accessibilityLabel("Goblins: pick a group")
        .popover(isPresented: $open, arrowEdge: .top) {
            GroupWeb(ui: model.config.ui, mode: "picker", payload: [
                "type": "picker",
                "items": Goblins.all().map(\.json),
                "selected": settings.goblinsPicked,
                "palette": palette.json,
            ]) { message in
                switch message["type"] as? String {
                case "toggle":
                    settings.goblinsPicked = message["selected"] as? [String] ?? []
                case "create":
                    let members = message["members"] as? [String] ?? []
                    guard !members.isEmpty else { return }
                    let names = members.compactMap { id in Goblins.all().first { $0.id == id }?.name }
                    let name = (message["name"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? GoblinGroups.defaultName(names)
                    let group = GoblinGroup(name: name, members: members)
                    settings.groups.insert(group, at: 0)
                    settings.goblinsPicked = []
                    model.page = .group(group.id)
                    open = false
                default: break
                }
            }
            .frame(width: 380, height: 440)
            .background(palette.content)
        }
    }
}

/// A group in the sidebar: its goblins in a dark circle, then its name.
struct GroupRow: View {
    var ui: URL
    var group: GoblinGroup
    var selected: Bool
    var action: () -> Void
    @Environment(\.palette) private var palette
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                GroupWeb(ui: ui, mode: "cluster", payload: group.payload(palette: palette), interactive: false)
                    .frame(width: 28, height: 28)
                Text(group.name).lineLimit(1).truncationMode(.tail)
                Spacer(minLength: 0)
            }
            .padding(.leading, 7)
            .padding(.trailing, 12)
            .frame(height: 40)
            .background {
                if selected {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(palette.sidebarText.opacity(palette.spec.dark ? 0.11 : 0.07))
                        .overlay { RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(palette.sidebarText.opacity(0.10), lineWidth: 0.5) }
                } else if hovering {
                    RoundedRectangle(cornerRadius: 10, style: .continuous).fill(palette.sidebarText.opacity(0.05))
                }
            }
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityLabel("Group \(group.name)")
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

/// The mock group chat, filling the card under its header.
struct GroupChatView: View {
    var ui: URL
    var group: GoblinGroup
    @Environment(\.palette) private var palette

    var body: some View {
        GroupWeb(ui: ui, mode: "chat", payload: group.payload(palette: palette))
            .id(group.id)
    }
}

extension Goblin {
    var json: [String: Any] { ["id": id, "name": name, "character": character] }
}

extension GoblinGroup {
    /// The group with its goblins' characters, for the page.
    func payload(palette: Palette) -> [String: Any] {
        let every = Goblins.all()
        let goblins = members.compactMap { id in every.first { $0.id == id } }
        return ["type": "group", "name": name, "members": goblins.map(\.json), "palette": palette.json]
    }
}

extension Palette {
    var json: [String: String] {
        ["content": spec.content, "text": spec.text, "secondary": spec.secondary, "accent": spec.accent, "bubble": spec.bubble]
    }
}

/// `web/group.html` in one of its modes. The payload is sent once the page is
/// ready and again when it changes; messages from the page go to `receive`.
struct GroupWeb: NSViewRepresentable {
    var ui: URL
    var mode: String
    var payload: [String: Any]
    var interactive = true
    var receive: ([String: Any]) -> Void = { _ in }

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> PassthroughWebView {
        let configuration = WKWebViewConfiguration()
        configuration.setURLSchemeHandler(UIFiles(root: ui), forURLScheme: "astrid-ui")
        configuration.userContentController.add(WeakHandler(context.coordinator), name: "astrid")
        let web = PassthroughWebView(frame: .zero, configuration: configuration)
        web.interactive = interactive
        web.setValue(false, forKey: "drawsBackground")
        context.coordinator.web = web
        context.coordinator.update(self)
        web.load(URLRequest(url: URL(string: "astrid-ui://app/group.html?mode=\(mode)")!))
        return web
    }

    func updateNSView(_ web: PassthroughWebView, context: Context) { context.coordinator.update(self) }

    static func dismantleNSView(_ web: PassthroughWebView, coordinator: Coordinator) {
        web.configuration.userContentController.removeScriptMessageHandler(forName: "astrid")
    }

    final class Coordinator: NSObject, WKScriptMessageHandler {
        weak var web: WKWebView?
        private var ready = false
        private var view: GroupWeb?
        private var sent: Data?

        func update(_ view: GroupWeb) {
            self.view = view
            guard ready, let data = try? JSONSerialization.data(withJSONObject: view.payload, options: [.sortedKeys]), data != sent else { return }
            sent = data
            if let json = String(data: data, encoding: .utf8) {
                web?.evaluateJavaScript("window.astrid && window.astrid.receive(\(json))")
            }
        }

        func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
            guard let body = message.body as? [String: Any] else { return }
            if body["type"] as? String == "ready" {
                ready = true
                sent = nil
                if let view { update(view) }
            } else {
                view?.receive(body)
            }
        }
    }
}
