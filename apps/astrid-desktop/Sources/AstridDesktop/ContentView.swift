import AppKit
import DesktopCore
import SwiftUI

/// The window, after the maintainer's Radian browser: a sidebar drawn on the
/// window's translucent material under a theme tint, and the main content as
/// an inset rounded card. There is one sidebar, and it always moves the same
/// way: it slides in from the left and slides back out. Opened (docked), it
/// pushes the card over; peeking (pointer on the left edge while closed), it
/// slides in the same way inside the card's box, over the card's content,
/// and the card stays put. The window buttons show only on a docked sidebar
/// wide enough to hold them. The sidebar toggle and new-chat buttons always
/// sit at the start of the card's header, open or closed; a peek draws the
/// same pair in the same spot over the header.
struct RootView: View {
    @ObservedObject var model: ChatModel
    @ObservedObject var settings: AppSettings
    @Environment(\.colorScheme) private var scheme
    @State private var peeking = false
    @State private var hidePeek: DispatchWorkItem?
    private let pinnedPeek = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_PEEK"] != nil
    /// Where AppKit put the window buttons, so the sidebar's top row lines up.
    @State private var lights = TrafficLights()

    /// One motion for opening, closing, peeking and un-peeking.
    static let slide = Animation.smooth(duration: 0.3)
    /// Height of the fixed button pair; its centre is on the window buttons'.
    static let row: CGFloat = 34

    var body: some View {
        let palette = Palette(spec: settings.theme(dark: scheme == .dark))
        let docked = settings.sidebarShown
        let visible = docked || peeking
        let width = settings.sidebarWidth
        // Docked, the sidebar's top is left to the window buttons.
        let lightsRow = lights.centerY + Self.row / 2 + 7
        ZStack(alignment: .leading) {
            ContentCard(model: model, settings: settings, peeking: peeking)
                .padding(.vertical, 8)
                .padding(.trailing, 8)
                .padding(.leading, docked ? width : 8)

            // Closed and not peeking: a strip on the left edge that peeks it.
            if !visible {
                Color.clear
                    .frame(width: 10)
                    .frame(maxHeight: .infinity)
                    .contentShape(.rect)
                    .onHover { if $0 { setPeek(true) } }
                    .accessibilityHidden(true)
            }

            // The sidebar slides inside a frame. Docked, the frame is the
            // window's left column; peeking, it is the card's own inset box
            // (same 8 pt margins, the card's rounded left corners), so the
            // sidebar stays within the card's bounds and never covers the
            // window margins.
            Sidebar(model: model, settings: settings, peeking: !docked, topInset: docked ? lightsRow : 0)
                .frame(width: width)
                .frame(maxHeight: .infinity)
                // Docked, the window's own tinted material shows through. Over
                // the card it needs the same look of its own, so it fades in
                // and out with the same motion.
                .background {
                    ZStack {
                        Rectangle().fill(.ultraThinMaterial)
                        palette.sidebar.opacity(min(1, palette.spec.sidebarOpacity + 0.2))
                    }
                    .opacity(docked ? 0 : 1)
                }
                .offset(x: visible ? 0 : -(width + 24))
                .frame(width: width, alignment: .leading)
                .clipShape(UnevenRoundedRectangle(topLeadingRadius: docked ? 0 : 12, bottomLeadingRadius: docked ? 0 : 12, style: .continuous))
                .shadow(color: .black.opacity(docked || !peeking ? 0 : (palette.spec.dark ? 0.3 : 0.12)), radius: 12, x: 3)
                .overlay(alignment: .trailing) {
                    if docked { ResizeHandle(width: $settings.sidebarWidth).offset(x: 4) }
                }
                .padding(docked ? 0 : 8)
                .allowsHitTesting(visible)
                .accessibilityHidden(!visible)
                .onHover { inside in if !docked { setPeek(inside) } }
        }
        .overlay {
            if let item = model.openDiagram {
                DiagramModal(model: model, item: item)
                    .transition(.opacity.combined(with: .scale(scale: 0.97)))
            }
        }
        .animation(.smooth(duration: 0.25), value: model.openDiagram?.id)
        .background { palette.sidebar.opacity(palette.spec.sidebarOpacity) }
        .background(WindowChrome(trafficLights: settings.trafficLightsVisible, measured: $lights))
        .ignoresSafeArea()
        .environment(\.palette, palette)
        .animation(Self.slide, value: docked)
        .animation(Self.slide, value: peeking)
        .sheet(isPresented: $settings.showSettings) {
            SettingsView(model: model, settings: settings)
                .environment(\.palette, palette)
                .tint(palette.accent)
                // Opaque, so the chat behind never shows through.
                .presentationBackground(palette.content)
        }
        // Docking from a peek keeps the sidebar where it is; only the card moves.
        .onChange(of: settings.sidebarShown) { _, shown in
            if shown && peeking { Haptics.settle() }
            if shown { peeking = false }
        }
        // Haptics along the slide, settling at full width.
        .onChange(of: visible) { _, nowVisible in Haptics.slide(opening: nowVisible) }
        // Debug: ASTRID_DESKTOP_PEEK=1 holds a closed sidebar peeking.
        .onAppear {
            if pinnedPeek {
                settings.sidebarShown = false
                peeking = true
            }
        }
        .onChange(of: settings.model) { _, _ in model.reconfigure(model: settings.model, mail: settings.mail) }
        .onChange(of: settings.mail) { _, _ in model.reconfigure(model: settings.model, mail: settings.mail) }
    }

    /// Shows at once; hides after a short grace so a brush past the edge
    /// doesn't flicker it.
    private func setPeek(_ on: Bool) {
        hidePeek?.cancel()
        if pinnedPeek { return }
        if on {
            peeking = true
        } else {
            let work = DispatchWorkItem { peeking = false }
            hidePeek = work
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3, execute: work)
        }
    }
}

/// Drag the docked sidebar's right edge to resize it.
struct ResizeHandle: View {
    @Binding var width: Double
    @State private var start: Double?

    var body: some View {
        Color.clear
            .frame(width: 8)
            .frame(maxHeight: .infinity)
            .contentShape(.rect)
            .onHover { inside in
                if inside { NSCursor.resizeLeftRight.push() } else { NSCursor.pop() }
            }
            .gesture(
                DragGesture(minimumDistance: 1, coordinateSpace: .global)
                    .onChanged { value in
                        let base = start ?? width
                        start = base
                        let next = min(max(base + value.translation.width, AppSettings.sidebarMin), AppSettings.sidebarMax)
                        // A detent every 20 pt; a firmer bump at either end.
                        if (next == AppSettings.sidebarMax || next == AppSettings.sidebarMin) && next != width {
                            Haptics.settle()
                        } else if Int(next / 20) != Int(width / 20) {
                            Haptics.tick()
                        }
                        width = next
                    }
                    .onEnded { _ in start = nil }
            )
            .accessibilityHidden(true)
    }
}

/// Where the window buttons sit, measured from the window's top-left corner.
struct TrafficLights: Equatable {
    /// The right edge of the zoom button, or nil when the buttons are hidden.
    var maxX: CGFloat? = 70
    var centerY: CGFloat = 16
}

/// Shows or hides the window's close, minimise and zoom buttons, and reports
/// where they are. Hidden buttons leave ⌘W, ⌘M and the Window menu working.
struct WindowChrome: NSViewRepresentable {
    var trafficLights: Bool
    @Binding var measured: TrafficLights

    func makeNSView(context: Context) -> NSView { NSView() }

    func updateNSView(_ view: NSView, context: Context) {
        let visible = trafficLights
        DispatchQueue.main.async {
            guard let window = view.window else { return }
            let buttons = [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton].compactMap { window.standardWindowButton($0) }
            if let zoom = buttons.last {
                let frame = zoom.convert(zoom.bounds, to: nil)
                let next = TrafficLights(maxX: frame.maxX, centerY: window.frame.height - frame.midY)
                if next != measured { measured = next }
            }
            NSAnimationContext.runAnimationGroup { context in
                context.duration = 0.2
                for button in buttons {
                    if visible { button.isHidden = false }
                    button.animator().alphaValue = visible ? 1 : 0
                }
            } completionHandler: {
                if !visible { buttons.forEach { $0.isHidden = true } }
            }
        }
    }
}

/// A round Liquid Glass toolbar button, like Radian's back and reload.
struct GlassCircle: View {
    var symbol: String
    var help: String
    var action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 14, weight: .medium))
                .frame(width: 20, height: 20)
        }
        .buttonStyle(.glass)
        .buttonBorderShape(.circle)
        .controlSize(.large)
        .help(help)
        .accessibilityLabel(help)
    }
}

struct Sidebar: View {
    @State private var renaming: ChatSession?
    @State private var newTitle = ""
    @ObservedObject var model: ChatModel
    @ObservedObject var settings: AppSettings
    /// Peeking over the card: the header's button pair is drawn again in the
    /// same spot, so the toggle can dock the sidebar.
    var peeking = false
    /// Room left at the top for the window buttons when docked.
    var topInset: CGFloat
    @Environment(\.palette) private var palette

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if peeking {
                // Same place as in the card header: 16 pt in, 50 pt tall.
                HStack {
                    SidebarButtons(model: model, settings: settings, peeking: true)
                    Spacer()
                }
                .padding(.leading, 4)
                .frame(height: 50)
                .padding(.bottom, -6)
            }
            SearchPill(query: $model.query) { settings.showSettings = true }

            VStack(spacing: 2) {
                SidebarRow(symbol: "plus", title: "New Chat", selected: false) { model.newChat() }
                SidebarRow(symbol: "square.grid.2x2", title: "Diagrams", selected: model.page == .diagrams) { model.page = .diagrams }
                Divider().opacity(0.5).padding(.vertical, 4)
                ScrollView {
                    LazyVStack(spacing: 2) {
                        ForEach(model.visibleChats) { chat in
                            SidebarRow(symbol: chat.icon.map(ChatGlyph.symbol) ?? (chat.messages.contains { $0.diagram != nil } ? "scribble.variable" : "bubble.left"),
                                       title: chat.title, selected: model.page == .chat && chat.id == model.selection,
                                       tint: ChatStyle.hex(chat.color).map { Color(hex: $0) }) { model.show(chat: chat.id) }
                                .help(chat.updated.formatted(.relative(presentation: .named)))
                                .contextMenu { ChatMenu(model: model, chat: chat) { renaming = chat; newTitle = chat.title } }
                        }
                        if model.visibleChats.isEmpty {
                            Text(model.chats.isEmpty ? "Your chats will show up here." : "No chats match.")
                                .font(.callout)
                                .foregroundStyle(palette.sidebarText.opacity(0.55))
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.horizontal, 12)
                                .padding(.top, 6)
                        }
                    }
                }
                .scrollIndicators(.never)
            }

            BlobsRow(ui: model.config.ui, settings: settings, model: model)
                .padding(.bottom, 4)
            HStack {
                AstridLogoMark(size: 21)
                    .frame(width: 38, height: 38)
                    .glassEffect(.regular, in: .circle)
                    .accessibilityHidden(true)
                Spacer()
                GlassCircle(symbol: "gearshape", help: "Settings (⌘,)") { settings.showSettings = true }
            }
            .padding(.bottom, 12)
        }
        .padding(.horizontal, 12)
        .padding(.top, topInset)
        .foregroundStyle(palette.sidebarText)
        .alert("Rename chat", isPresented: Binding(get: { renaming != nil }, set: { if !$0 { renaming = nil } })) {
            TextField("Name", text: $newTitle)
            Button("Rename") { if let chat = renaming { model.restyle(chat.id, title: newTitle) } }
            Button("Cancel", role: .cancel) {}
        }
    }
}

/// SF Symbols for `ChatStyle.icons`.
enum ChatGlyph {
    static func symbol(_ name: String) -> String {
        [
            "chat": "bubble.left", "star": "star", "brain": "brain.head.profile", "code": "chevron.left.forwardslash.chevron.right",
            "book": "book.closed", "heart": "heart", "bolt": "bolt", "globe": "globe", "flask": "flask", "music": "music.note",
            "house": "house", "camera": "camera", "map": "map", "leaf": "leaf",
        ][name] ?? "bubble.left"
    }
}

/// A chat's right-click menu: rename, icon and colour.
struct ChatMenu: View {
    @ObservedObject var model: ChatModel
    var chat: ChatSession
    var rename: () -> Void

    var body: some View {
        Button("Rename…", systemImage: "pencil", action: rename)
        Menu("Icon") {
            Button("Default") { model.restyle(chat.id, icon: "") }
            Divider()
            ForEach(ChatStyle.icons, id: \.self) { icon in
                Button(icon.capitalized, systemImage: ChatGlyph.symbol(icon)) { model.restyle(chat.id, icon: icon) }
            }
        }
        Menu("Colour") {
            Button("Default") { model.restyle(chat.id, color: "") }
            Divider()
            ForEach(ChatStyle.colors, id: \.name) { color in
                Button {
                    model.restyle(chat.id, color: color.name)
                } label: {
                    Label {
                        Text(color.name.capitalized)
                    } icon: {
                        Image(nsImage: ChatMenu.swatch(color.hex))
                    }
                }
            }
        }
    }

    /// Menus draw images, not shapes: a small round swatch.
    static func swatch(_ hex: String) -> NSImage {
        NSImage(size: NSSize(width: 12, height: 12), flipped: false) { rect in
            NSColor(Color(hex: hex)).setFill()
            NSBezierPath(ovalIn: rect.insetBy(dx: 1, dy: 1)).fill()
            return true
        }
    }
}

/// The sidebar toggle and new chat, side by side in one fixed spot whatever
/// the sidebar is doing. The toggle opens a closed sidebar, docks a peeking
/// one and closes a docked one.
struct SidebarButtons: View {
    @ObservedObject var model: ChatModel
    @ObservedObject var settings: AppSettings
    var peeking: Bool

    var body: some View {
        HStack(spacing: 8) {
            GlassCircle(symbol: "sidebar.left", help: settings.sidebarShown ? "Hide Sidebar (⌃⌘S)" : peeking ? "Keep Sidebar Open (⌃⌘S)" : "Show Sidebar (⌃⌘S)") {
                settings.sidebarShown.toggle()
            }
            GlassCircle(symbol: "square.and.pencil", help: "New Chat (⌘N)") { model.newChat() }
        }
    }
}

/// Radian's address field, here searching chats; the sliders open Settings.
struct SearchPill: View {
    @Binding var query: String
    var openSettings: () -> Void
    @Environment(\.palette) private var palette

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(palette.sidebarText.opacity(0.6))
            TextField("Search chats", text: $query)
                .textFieldStyle(.plain)
            if !query.isEmpty {
                Button { query = "" } label: { Image(systemName: "xmark.circle.fill") }
                    .buttonStyle(.plain)
                    .foregroundStyle(palette.sidebarText.opacity(0.5))
                    .accessibilityLabel("Clear search")
            }
            Button(action: openSettings) { Image(systemName: "slider.horizontal.3") }
                .buttonStyle(.plain)
                .help("Settings")
                .accessibilityLabel("Settings")
        }
        .padding(.horizontal, 12)
        .frame(height: 38)
        .glassEffect(.regular, in: .rect(cornerRadius: 13))
    }
}

/// One sidebar row; the selected one is a raised, lightly outlined capsule.
struct SidebarRow: View {
    var symbol: String
    var title: String
    var selected: Bool
    var tint: Color? = nil
    var action: () -> Void
    @Environment(\.palette) private var palette
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: symbol)
                    .font(.system(size: 13))
                    .frame(width: 18)
                    .foregroundStyle(tint ?? palette.sidebarText)
                    .opacity(tint == nil ? 0.8 : 1)
                Text(title)
                    .lineLimit(1)
                    .truncationMode(.tail)
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 12)
            .frame(height: 36)
            .background {
                if selected {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(palette.sidebarText.opacity(palette.spec.dark ? 0.11 : 0.07))
                        .overlay {
                            RoundedRectangle(cornerRadius: 10, style: .continuous)
                                .strokeBorder(palette.sidebarText.opacity(0.10), lineWidth: 0.5)
                        }
                        .shadow(color: .black.opacity(palette.spec.dark ? 0.25 : 0.08), radius: 2, y: 1)
                } else if hovering {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(palette.sidebarText.opacity(0.05))
                }
            }
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

/// The main content: an inset rounded card in the theme's content colour.
struct ContentCard: View {
    @ObservedObject var model: ChatModel
    @ObservedObject var settings: AppSettings
    var peeking = false
    @Environment(\.palette) private var palette

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                // The toggle and new chat, in the same spot open or closed.
                SidebarButtons(model: model, settings: settings, peeking: peeking)
                Text(model.page == .diagrams ? "Diagrams" : model.selected?.title ?? "New Chat")
                    .font(.headline)
                    .lineLimit(1)
                if model.page == .diagrams {
                    Text("\(model.diagrams.count)")
                        .font(.callout.monospacedDigit())
                        .foregroundStyle(palette.secondary)
                }
                Spacer()
            }
            .padding(.horizontal, 16)
            .frame(height: 50)
            switch model.page {
            case .chat: ThreadView(model: model)
            case .diagrams: DiagramsView(model: model)
            }
        }
        .foregroundStyle(palette.text)
        .background(palette.content)
        .clipShape(.rect(cornerRadius: 12, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .strokeBorder(palette.text.opacity(0.07), lineWidth: 0.5)
        }
        .shadow(color: .black.opacity(palette.spec.dark ? 0.35 : 0.10), radius: 10, y: 2)
    }
}

struct ThreadView: View {
    @ObservedObject var model: ChatModel

    var body: some View {
        let thread = model.thread
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 22) {
                    ForEach(thread) { message in
                        MessageRow(model: model, message: message)
                            .id(message.id)
                    }
                }
                .frame(maxWidth: 760)
                .padding(.horizontal, 28)
                .padding(.top, 12)
                .padding(.bottom, 12)
                .frame(maxWidth: .infinity)
            }
            .defaultScrollAnchor(.bottom)
            .scrollEdgeEffectStyle(.soft, for: .bottom)
            // A bar the thread scrolls under, with the system's soft edge.
            .safeAreaBar(edge: .bottom) { Composer(model: model) }
            .onChange(of: thread.last?.id) { _, id in
                withAnimation(.smooth) { proxy.scrollTo(id, anchor: .bottom) }
            }
            .onChange(of: model.live?.streamed.count) { _, _ in
                proxy.scrollTo(thread.last?.id, anchor: .bottom)
            }
        }
        .overlay {
            if thread.isEmpty { EmptyThread(ui: model.config.ui, draft: model.draft) }
        }
        // Voice mode floats over the thread with nothing behind it: inside the
        // composer bar it took the bar's backdrop with it.
        .overlay(alignment: .bottom) { VoiceLayer(model: model, voice: model.voice) }
    }
}

/// Astrid's mark, from the web UI's favicon so both apps share one source.
struct AstridMark: View {
    var ui: URL
    var size: CGFloat

    var body: some View {
        if let image = NSImage(contentsOf: ui.appendingPathComponent("favicon.svg")) {
            Image(nsImage: image)
                .renderingMode(.template)
                .resizable()
                .aspectRatio(contentMode: .fit)
                .frame(width: size)
                .accessibilityHidden(true)
        }
    }
}

/// Voice mode's panel, when it's on.
struct VoiceLayer: View {
    @ObservedObject var model: ChatModel
    @ObservedObject var voice: VoiceMode

    var body: some View {
        ZStack {
            if voice.phase != .off {
                VoicePanel(ui: model.config.ui, voice: voice)
                    .frame(maxWidth: 760)
                    .padding(.horizontal, 28)
                    .padding(.bottom, 6)
                    .transition(.opacity.combined(with: .scale(scale: 0.92, anchor: .bottom)))
            }
        }
        .animation(.smooth(duration: 0.35), value: voice.phase == .off)
    }
}

struct EmptyThread: View {
    var ui: URL
    var draft = ""
    @Environment(\.palette) private var palette

    var body: some View {
        VStack(spacing: 14) {
            TrackingLogoMark(size: 64, draft: draft)
            Text("Ask Astrid")
                .font(.title2.weight(.semibold))
            Text("Each chat keeps what you sent, Astrid's replies\nand the drawings she made.")
                .multilineTextAlignment(.center)
                .foregroundStyle(palette.secondary)
        }
        .padding(.bottom, 60)
    }
}

struct MessageRow: View {
    @ObservedObject var model: ChatModel
    var message: ChatMessage
    @Environment(\.palette) private var palette

    private var live: LiveReply? {
        model.live.flatMap { $0.id == message.id ? $0 : nil }
    }

    var body: some View {
        switch message.role {
        case .user:
            HStack {
                Spacer(minLength: 90)
                Text(message.text)
                    .textSelection(.enabled)
                    .padding(.horizontal, 15)
                    .padding(.vertical, 10)
                    .background(palette.bubble, in: .rect(cornerRadius: 18))
            }
        case .astrid:
            reply
        }
    }

    private var reply: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 7) {
                AstridMark(ui: model.config.ui, size: 18)
                Text("Astrid").font(.callout.weight(.semibold))
                if let live {
                    Text(live.drawing ? "Sketching" : live.label)
                        .font(.caption)
                        .foregroundStyle(palette.secondary)
                        .padding(.horizontal, 9)
                        .padding(.vertical, 3)
                        .glassEffect(.regular, in: .capsule)
                        .contentTransition(.opacity)
                        .animation(.smooth, value: live.label)
                }
            }
            if !message.text.isEmpty {
                Text(markdown(message.text))
                    .textSelection(.enabled)
                    .lineSpacing(3)
            }
            if let brief = message.brief { BriefView(brief: brief) }
            if let shapes = message.diagram, !shapes.isEmpty {
                DiagramView(ui: model.config.ui, shapes: shapes, animate: model.animated.contains(message.id), finished: live.map { !$0.drawing } ?? true)
                    .frame(height: 420)
                    .clipShape(.rect(cornerRadius: 18))
            }
            if let approval = live?.approval {
                ApprovalCard(title: approval.title) { model.answerApproval($0) }
            }
            if live == nil, let outcome = message.outcome, outcome != "completed" {
                Label(outcome == "cancelled" ? "Stopped" : outcome == "limit" ? "Hit its step limit" : "Didn't finish", systemImage: "exclamationmark.circle")
                    .font(.caption)
                    .foregroundStyle(palette.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func markdown(_ text: String) -> AttributedString {
        (try? AttributedString(markdown: text, options: .init(interpretedSyntax: .inlineOnlyPreservingWhitespace))) ?? AttributedString(text)
    }
}

/// A reply's brief (ADR 0033), as a quiet card under the text.
struct BriefView: View {
    var brief: JSONValue
    @Environment(\.palette) private var palette

    private func texts(_ key: String) -> [String] {
        brief[key]?.array?.compactMap { $0["text"]?.string } ?? []
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let title = brief["title"]?.string { Text(title).font(.headline) }
            if let summary = brief["summary"]?.string { Text(summary).foregroundStyle(palette.secondary) }
            ForEach(Array(texts("todos").enumerated()), id: \.offset) { _, text in
                Label(text, systemImage: "circle")
            }
            ForEach(Array(texts("updates").enumerated()), id: \.offset) { index, text in
                Text("\(index + 1). \(text)")
            }
            let sources = brief["sources"]?.array ?? []
            if !sources.isEmpty {
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(Array(sources.enumerated()), id: \.offset) { index, source in
                        let from = source["from"]?.string ?? "Unknown sender"
                        let subject = source["subject"]?.string ?? ""
                        Text("[\(index + 1)] \(from) · \(subject)")
                            .lineLimit(1)
                    }
                }
                .font(.caption)
                .foregroundStyle(palette.secondary)
            }
        }
        .textSelection(.enabled)
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(palette.text.opacity(0.05), in: .rect(cornerRadius: 14))
    }
}

struct ApprovalCard: View {
    var title: String
    var decide: (Bool) -> Void
    @Environment(\.palette) private var palette

    var body: some View {
        GlassEffectContainer(spacing: 8) {
            HStack(spacing: 10) {
                Image(systemName: "envelope.badge.shield.half.filled")
                    .foregroundStyle(palette.secondary)
                Text(title).lineLimit(2)
                Spacer(minLength: 8)
                Button("Don't") { decide(false) }
                    .buttonStyle(.glass)
                Button("Allow once") { decide(true) }
                    .buttonStyle(.glassProminent)
                    .tint(palette.accent)
                    .keyboardShortcut(.defaultAction)
            }
            .padding(12)
            .glassEffect(.regular, in: .rect(cornerRadius: 16))
        }
    }
}

struct Composer: View {
    @ObservedObject var model: ChatModel
    @ObservedObject private var voice: VoiceMode
    @FocusState private var focused: Bool
    @Environment(\.palette) private var palette

    init(model: ChatModel) {
        self.model = model
        voice = model.voice
    }

    private var running: Bool { model.live != nil }
    private var elsewhere: Bool { model.live.map { $0.chatID != model.selection } ?? false }
    private var typing: Bool { !model.draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
    private var talking: Bool { voice.phase != .off }

    var body: some View {
        VStack(spacing: 6) {
            if elsewhere {
                Text("Astrid is answering in another chat.")
                    .font(.caption)
                    .foregroundStyle(palette.secondary)
            }
            if let error = model.saveError {
                Text(error).font(.caption).foregroundStyle(.red)
            }
            if let problem = voice.problem {
                Text(problem).font(.caption).foregroundStyle(.red)
            }
            // The field stays in the window while she talks (hidden): taking a
            // focused field out sends AppKit's key-view search round forever.
            GlassEffectContainer(spacing: 10) {
                HStack(alignment: .bottom, spacing: 10) {
                    TextField("Ask Astrid", text: $model.draft, axis: .vertical)
                        .textFieldStyle(.plain)
                        .lineLimit(1...8)
                        .focused($focused)
                        .onSubmit { model.send() }
                        .padding(.horizontal, 16)
                        .padding(.vertical, 11)
                        .opacity(talking ? 0 : 1)
                        .glassEffect(talking ? .identity : .regular.interactive(), in: .rect(cornerRadius: 20))
                    if !talking { button }
                }
                .opacity(talking ? 0 : 1)
                .disabled(talking)
                .accessibilityHidden(talking)
            }
            .animation(.smooth(duration: 0.35), value: talking)
            .animation(.smooth(duration: 0.2), value: typing)
        }
        .frame(maxWidth: 760)
        .padding(.horizontal, 28)
        .padding(.bottom, 16)
        .padding(.top, 6)
        .frame(maxWidth: .infinity)
        .onAppear { focused = true }
        .onChange(of: model.selection) { _, _ in if !talking { focused = true } }
        .onChange(of: talking) { _, on in if !on { focused = true } }
        .onKeyPress(.escape) {
            guard talking else { return .ignored }
            voice.end()
            return .handled
        }
    }

    @ViewBuilder private var button: some View {
        if running {
            Button { model.cancel() } label: {
                Image(systemName: "stop.fill").frame(width: 22, height: 22)
            }
            .buttonStyle(.glass)
            .buttonBorderShape(.circle)
            .controlSize(.large)
            .keyboardShortcut(.cancelAction)
            .help("Stop (Esc)")
        } else if typing {
            Button { model.send() } label: {
                Image(systemName: "arrow.up").fontWeight(.semibold).frame(width: 22, height: 22)
            }
            .buttonStyle(.glassProminent)
            .tint(palette.accent)
            .buttonBorderShape(.circle)
            .controlSize(.large)
            .disabled(!model.canSend)
            .help("Send (Return)")
        } else {
            Button {
                // Let go of the field first, then talk.
                focused = false
                NSApp.keyWindow?.makeFirstResponder(nil)
                DispatchQueue.main.async { voice.start() }
            } label: {
                Image(systemName: "waveform").fontWeight(.semibold).frame(width: 22, height: 22)
            }
            .buttonStyle(.glassProminent)
            .tint(palette.accent)
            .buttonBorderShape(.circle)
            .controlSize(.large)
            .help("Talk to Astrid")
        }
    }
}

/// The composer in voice mode: Astrid as an orb in her voice ring, which
/// moves with your voice while she listens and with hers while she speaks;
/// mute on the left, back to the keyboard on the right. Tap her to interrupt.
struct VoicePanel: View {
    var ui: URL
    @ObservedObject var voice: VoiceMode
    @Environment(\.palette) private var palette

    private var label: String {
        if voice.muted && voice.phase == .listening { return "Muted" }
        switch voice.phase {
        case .listening: return voice.heard.isEmpty ? "Listening…" : voice.heard
        case .thinking: return "Thinking…"
        case .speaking: return "Tap to interrupt"
        case .off: return ""
        }
    }

    var body: some View {
        ZStack(alignment: .bottom) {
            VStack(spacing: 8) {
                VoiceRingView(ui: ui, voice: voice)
                    .frame(width: 210, height: 210)
                    .accessibilityLabel("Astrid, \(label)")
                Text(label)
                    .font(.callout)
                    .lineLimit(2)
                    .multilineTextAlignment(.center)
                    .foregroundStyle(voice.phase == .listening && !voice.heard.isEmpty ? palette.text : palette.secondary)
                    .frame(maxWidth: 420, minHeight: 36, alignment: .top)
                    .contentTransition(.opacity)
                    .animation(.smooth, value: label)
            }
            .frame(maxWidth: .infinity)
            HStack {
                Button { voice.toggleMute() } label: {
                    Image(systemName: voice.muted ? "mic.slash.fill" : "mic.fill").frame(width: 24, height: 24)
                }
                .buttonStyle(.glass)
                .buttonBorderShape(.circle)
                .controlSize(.extraLarge)
                .help(voice.muted ? "Unmute" : "Mute")
                Spacer()
                Button { voice.end() } label: {
                    Image(systemName: "keyboard").frame(width: 24, height: 24)
                }
                .buttonStyle(.glass)
                .buttonBorderShape(.circle)
                .controlSize(.extraLarge)
                .help("Back to typing (Esc)")
            }
            .padding(.bottom, 44)
        }
    }
}

