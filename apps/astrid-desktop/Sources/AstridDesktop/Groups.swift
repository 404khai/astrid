import AppKit
import AstridWebUI
import DesktopCore
import SwiftUI
import WebKit

/// Blob groups (cosmetic for now, ADR 0037 amendment): pick blobs with the
/// Giovanni button in the composer, make a group, and it shows in the sidebar
/// as its blobs in a dark circle with its name, opening a mock group chat.
/// Everything is drawn by `web/group.html`; no model is involved.

/// Giovanni as a macOS app icon: himself, in his own colours, on a deep
/// squircle with a soft top light.
struct GiovanniIcon: View {
    var size: CGFloat

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: size * 0.27, style: .continuous)
                .fill(LinearGradient(colors: [Color(hex: "#3B3F9E"), Color(hex: "#1D1F55")], startPoint: .top, endPoint: .bottom))
            RoundedRectangle(cornerRadius: size * 0.27, style: .continuous)
                .fill(LinearGradient(colors: [.white.opacity(0.22), .clear], startPoint: .top, endPoint: .center))
            if let icon = Blobs.icon {
                Image(nsImage: icon)
                    .renderingMode(.original)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .padding(size * 0.12)
                    .shadow(color: .black.opacity(0.35), radius: 1, y: 0.8)
            }
        }
        .frame(width: size, height: size)
        .shadow(color: .black.opacity(0.25), radius: 1.5, y: 1)
        .accessibilityHidden(true)
    }
}

/// The composer's blob button, beside the waveform: opens the blob picker.
struct BlobButton: View {
    @ObservedObject var model: ChatModel
    @EnvironmentObject private var settings: AppSettings
    @State private var open = false
    @Environment(\.palette) private var palette

    var body: some View {
        Button { open.toggle() } label: {
            GiovanniIcon(size: 24).frame(width: 22, height: 22)
        }
        .buttonStyle(.glass)
        .buttonBorderShape(.circle)
        .controlSize(.large)
        .help("Blobs")
        .accessibilityLabel("Blobs: pick a group")
        .popover(isPresented: $open, arrowEdge: .top) {
            GroupWeb(ui: model.config.ui, mode: "picker", payload: [
                "type": "picker",
                "items": Blobs.all().map(\.json),
                "selected": settings.blobsPicked,
                "palette": palette.json,
            ]) { message in
                switch message["type"] as? String {
                case "toggle":
                    settings.blobsPicked = message["selected"] as? [String] ?? []
                case "create":
                    let members = message["members"] as? [String] ?? []
                    guard !members.isEmpty else { return }
                    let names = members.compactMap { id in Blobs.all().first { $0.id == id }?.name }
                    let name = (message["name"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? BlobGroups.defaultName(names)
                    let group = BlobGroup(name: name, members: members)
                    settings.groups.insert(group, at: 0)
                    settings.blobsPicked = []
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

/// A group in the sidebar: its blobs in a dark circle, then its name.
struct GroupRow: View {
    var ui: URL
    var group: BlobGroup
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
    var group: BlobGroup
    @Environment(\.palette) private var palette

    var body: some View {
        GroupWeb(ui: ui, mode: "chat", payload: group.payload(palette: palette))
            .id(group.id)
    }
}

extension Blob {
    var json: [String: Any] { ["id": id, "name": name, "character": character] }
}

extension BlobGroup {
    /// The group with its blobs' characters, for the page.
    func payload(palette: Palette) -> [String: Any] {
        let every = Blobs.all()
        let blobs = members.compactMap { id in every.first { $0.id == id } }
        return ["type": "group", "name": name, "members": blobs.map(\.json), "palette": palette.json]
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
