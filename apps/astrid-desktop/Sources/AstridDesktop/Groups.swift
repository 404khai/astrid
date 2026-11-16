import AppKit
import AstridWebUI
import DesktopCore
import SwiftUI
import WebKit

/// Goblin groups (cosmetic for now, ADR 0037 amendment): pick goblins with the
/// Giovanni button in the composer, make a group, and it shows in the sidebar
/// as its goblins in a dark circle with its name, opening a mock group chat.
/// Everything is drawn by `web/group.html`; no model is involved.

/// Bramble as a symbol-style glyph, like an SF Symbol, from the maintainer's
/// recipe (wide head, long ears, bulb nose, beady eyes, gnome hat, no beard,
/// no headphones): white outlines, solid eyes, his idle smile. Drawn natively
/// from the vendored renderer's geometry (web/src/goblin/svg.ts, 1 unit = 60,
/// head centred at 120, 150). Each part clears what is under it, so no lines
/// cross.
struct BrambleGlyph: View {
    var size: CGFloat
    var color: Color = .white

    var body: some View {
        Canvas { context, canvas in
            // The drawing spans x -15…255 and y 5…202; fit its width.
            let s = canvas.width / 282
            let ox = canvas.width / 2 - 120 * s, oy = canvas.height / 2 - 104 * s
            func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: ox + x * s, y: oy + y * s) }
            func oval(_ cx: CGFloat, _ cy: CGFloat, _ rx: CGFloat, _ ry: CGFloat) -> Path {
                Path(ellipseIn: CGRect(x: ox + (cx - rx) * s, y: oy + (cy - ry) * s, width: rx * 2 * s, height: ry * 2 * s))
            }
            let line = StrokeStyle(lineWidth: max(1.4, 14 * s), lineCap: .round, lineJoin: .round)
            context.drawLayer { layer in
                func part(_ path: Path) {
                    layer.blendMode = .clear
                    layer.fill(path, with: .color(.black))
                    layer.blendMode = .normal
                    layer.stroke(path, with: .color(color), style: line)
                }
                // Long ears, angled up and out (the renderer's "long" ear).
                for mirror in [false, true] {
                    func m(_ x: CGFloat, _ y: CGFloat) -> CGPoint { p(mirror ? 240 - x : x, y) }
                    var ear = Path()
                    ear.move(to: m(172, 134))
                    ear.addQuadCurve(to: m(255, 116), control: m(216, 119))
                    ear.addQuadCurve(to: m(180, 157), control: m(223, 140))
                    ear.closeSubpath()
                    part(ear)
                }
                // Wide head.
                part(oval(120, 150, 72, 52))
                // Gnome hat, leaning right.
                var hat = Path()
                hat.move(to: p(45, 125))
                hat.addQuadCurve(to: p(156, 5), control: p(79, 47))
                hat.addQuadCurve(to: p(195, 125), control: p(184, 59))
                hat.addQuadCurve(to: p(45, 125), control: p(120, 133))
                hat.closeSubpath()
                part(hat)
                // Beady eyes, bulb nose, idle smile.
                layer.fill(oval(94, 143, 8, 9.5), with: .color(color))
                layer.fill(oval(146, 143, 8, 9.5), with: .color(color))
                part(oval(120, 157, 14, 12.5))
                var smile = Path()
                smile.move(to: p(108, 175))
                smile.addQuadCurve(to: p(132, 175), control: p(120, 186))
                layer.stroke(smile, with: .color(color), style: line)
            }
        }
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
