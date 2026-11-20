import AppKit
import AstridWebUI
import DesktopCore
import SwiftUI
import WebKit

/// Goblin groups (cosmetic for now, ADR 0037 amendment): pick goblins with the
/// Giovanni button in the composer, make a group, and it shows in the sidebar
/// as its goblins in a dark circle with its name, opening a mock group chat.
/// Everything is drawn by `web/group.html`; group chats run through
/// `GroupChatModel` (amendment 14).

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

/// A group chat, filling the card under its header: the thread drawn by
/// `web/group.html`, and the native Liquid Glass composer over it. The side
/// panel renames the group, describes it (its purpose, in every prompt) and
/// sets each goblin's role.
struct GroupChatView: View {
    var ui: URL
    var group: GoblinGroup
    @ObservedObject var chat: GroupChatModel
    @EnvironmentObject private var settings: AppSettings
    @Environment(\.palette) private var palette

    private var goblins: [Goblin] {
        let every = Goblins.all()
        return group.members.compactMap { id in every.first { $0.id == id } }
    }

    var body: some View {
        let goblins = self.goblins
        let members = goblins.map { GroupMember(id: $0.id, name: $0.name) }
        ZStack(alignment: .bottom) {
            GroupWeb(ui: ui, mode: "chat", payload: payload(goblins)) { message in receive(message, members: members) }
            GroupComposer(chat: chat, group: group, members: members, ui: ui, goblins: goblins)
        }
        .id(group.id)
    }

    private func payload(_ goblins: [Goblin]) -> [String: Any] {
        let thread = chat.thread(group.id)
        var out = group.payload(palette: palette, goblins: goblins)
        out["messages"] = thread.messages.map(\.json)
        out["roles"] = group.roles ?? [:]
        out["astrid"] = (chat.overseer[group.id] ?? .idle).rawValue
        out["panel"] = chat.panelRequest
        out["memory"] = Dictionary(uniqueKeysWithValues: goblins.map { ($0.id, chat.memory($0.id).facts.map(\.text)) })
        if let live = chat.live, live.group == group.id {
            out["live"] = ["speaker": live.speaker, "text": GroupReply.visible(live.streamed)]
        }
        return out
    }

    private func receive(_ message: [String: Any], members: [GroupMember]) {
        switch message["type"] as? String {
        case "save":
            guard let index = settings.groups.firstIndex(where: { $0.id == group.id }) else { return }
            let name = (message["name"] as? String ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            let about = (message["about"] as? String ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            var saved = settings.groups[index]
            if !name.isEmpty { saved.name = String(name.prefix(60)) }
            saved.about = about.isEmpty ? nil : String(about.prefix(GoblinGroup.maxAbout))
            for (member, role) in message["roles"] as? [String: String] ?? [:] where saved.members.contains(member) {
                let before = saved.role(of: member)
                saved.setRole(role, for: member)
                if let now = saved.role(of: member), now != before { chat.noteRole(now, for: member, in: saved.name) }
            }
            settings.groups[index] = saved
        case "react":
            guard let id = (message["id"] as? String).flatMap(UUID.init), let emoji = message["emoji"] as? String else { return }
            chat.react(emoji, to: id, in: group.id)
        case "mention":
            // A name tapped in the thread goes into the composer.
            guard let name = message["name"] as? String else { return }
            let draft = chat.drafts[group.id] ?? ""
            chat.drafts[group.id] = draft + (draft.isEmpty || draft.hasSuffix(" ") ? "" : " ") + "@\(name) "
        case "forget":
            if let goblin = message["goblin"] as? String { chat.clearMemory(goblin) }
        default: break
        }
    }
}

/// The group's composer, Liquid Glass like the chat's: type, tag with @ (the
/// names show above the field as you type one), send or stop.
struct GroupComposer: View {
    @ObservedObject var chat: GroupChatModel
    var group: GoblinGroup
    var members: [GroupMember]
    @FocusState private var focused: Bool
    @Environment(\.palette) private var palette

    private var draft: Binding<String> {
        Binding(get: { chat.drafts[group.id] ?? "" }, set: { chat.drafts[group.id] = $0 })
    }
    private var running: Bool { chat.busy(group.id) }
    private var typing: Bool { !draft.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

    var ui: URL
    var goblins: [Goblin]

    /// Members matching the `@word` being typed at the end of the draft.
    private var suggestions: [String] {
        guard let last = draft.wrappedValue.split(separator: " ", omittingEmptySubsequences: false).last, last.hasPrefix("@") else { return [] }
        let typed = last.dropFirst().lowercased()
        return (["all", "Astrid"] + members.map(\.name)).filter { typed.isEmpty || $0.lowercased().hasPrefix(typed) && $0.lowercased() != typed }
    }

    /// The face beside a suggestion: the goblin, every goblin for @all, Astrid's mark.
    @ViewBuilder private func face(_ name: String) -> some View {
        if name == "Astrid" {
            AstridLogoMark(size: 18)
        } else {
            let shown = name == "all" ? goblins : goblins.filter { $0.name == name }.prefix(1).map { $0 }
            GroupWeb(ui: ui, mode: "cluster", payload: ["type": "group", "name": name, "members": shown.map(\.json)], interactive: false)
                .frame(width: 20, height: 20)
        }
    }

    var body: some View {
        VStack(spacing: 8) {
            if let problem = chat.problem {
                Text(problem).font(.caption).foregroundStyle(.red)
            }
            if !suggestions.isEmpty {
                GlassEffectContainer(spacing: 6) {
                    HStack(spacing: 6) {
                        ForEach(suggestions, id: \.self) { name in
                            Button { complete(name) } label: {
                                HStack(spacing: 5) {
                                    face(name)
                                    Text("@\(name)")
                                }
                            }
                                .buttonStyle(.glass)
                                .controlSize(.small)
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .transition(.opacity.combined(with: .move(edge: .bottom)))
            }
            GlassEffectContainer(spacing: 10) {
                HStack(alignment: .bottom, spacing: 10) {
                    TextField("Message \(group.name), @ to tag", text: draft, axis: .vertical)
                        .textFieldStyle(.plain)
                        .lineLimit(1...6)
                        .focused($focused)
                        .onSubmit(send)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 11)
                        .glassEffect(.regular.interactive(), in: .rect(cornerRadius: 20))
                    if running {
                        Button { chat.stop() } label: {
                            Image(systemName: "stop.fill").frame(width: 22, height: 22)
                        }
                        .buttonStyle(.glass)
                        .buttonBorderShape(.circle)
                        .controlSize(.large)
                        .keyboardShortcut(.cancelAction)
                        .help("Stop the goblins (Esc)")
                    } else {
                        Button(action: send) {
                            Image(systemName: "arrow.up").fontWeight(.semibold).frame(width: 22, height: 22)
                        }
                        .buttonStyle(.glassProminent)
                        .tint(palette.accent)
                        .buttonBorderShape(.circle)
                        .controlSize(.large)
                        .disabled(!typing || chat.live != nil)
                        .help("Send (Return)")
                    }
                }
            }
        }
        .animation(.smooth(duration: 0.2), value: suggestions)
        .frame(maxWidth: 760)
        .padding(.horizontal, 28)
        .padding(.bottom, 16)
        .frame(maxWidth: .infinity)
        .onAppear { focused = true }
    }

    private func send() {
        chat.send(in: group, members: members)
        focused = true
    }

    private func complete(_ name: String) {
        var words = draft.wrappedValue.split(separator: " ", omittingEmptySubsequences: false).map(String.init)
        words.removeLast()
        draft.wrappedValue = (words + ["@\(name) "]).joined(separator: " ")
        focused = true
    }
}

extension GroupMessage {
    var json: [String: Any] {
        [
            "id": id.uuidString, "author": author, "text": text,
            "at": at.formatted(date: .omitted, time: .shortened),
            "mood": mood ?? "talking", "mentions": mentions,
            "reactions": reactions.map { ["emoji": $0.key, "by": $0.value] }.sorted { ($0["emoji"] as! String) < ($1["emoji"] as! String) },
            "outcome": outcome as Any,
            "chart": chart?.any as Any,
        ]
    }
}

extension Goblin {
    var json: [String: Any] { ["id": id, "name": name, "character": character] }
}

extension GoblinGroup {
    /// The group with its goblins' characters, for the page.
    func payload(palette: Palette, goblins given: [Goblin]? = nil) -> [String: Any] {
        let every = given ?? Goblins.all()
        let goblins = members.compactMap { id in every.first { $0.id == id } }
        return ["type": "group", "name": name, "about": about ?? "", "members": goblins.map(\.json), "palette": palette.json]
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
        // Debug: ASTRID_DESKTOP_GROUP=panel opens the chat with its side panel.
        let panel = mode == "chat" && ProcessInfo.processInfo.environment["ASTRID_DESKTOP_GROUP"] == "panel" ? "&panel=1" : ""
        web.load(URLRequest(url: URL(string: "astrid-ui://app/group.html?mode=\(mode)\(panel)")!))
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
