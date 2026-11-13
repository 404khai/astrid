import AppKit
import DesktopCore
import SwiftUI

/// The window, after the maintainer's Radian browser: a sidebar drawn on the
/// window's translucent material under a theme tint, and the main content as
/// an inset rounded card. Collapsing the sidebar lets the card fill the window;
/// then, as in Arc, resting the pointer on the left edge peeks the sidebar over
/// the content. The window buttons show only on a docked sidebar wide enough
/// to hold them.
struct RootView: View {
    @ObservedObject var model: ChatModel
    @ObservedObject var settings: AppSettings
    @Environment(\.colorScheme) private var scheme
    @State private var peeking = false
    @State private var hidePeek: DispatchWorkItem?
    private let pinnedPeek = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_PEEK"] != nil
    /// Where AppKit put the window buttons, so the sidebar's top row lines up.
    @State private var lights = TrafficLights()

    var body: some View {
        let palette = Palette(spec: settings.theme(dark: scheme == .dark))
        HStack(spacing: 0) {
            if settings.sidebarShown {
                Sidebar(model: model, settings: settings, lights: settings.trafficLightsVisible ? lights : TrafficLights(maxX: nil, centerY: lights.centerY))
                    .frame(width: settings.sidebarWidth)
                    .overlay(alignment: .trailing) { ResizeHandle(width: $settings.sidebarWidth).offset(x: 4) }
                    .transition(.move(edge: .leading).combined(with: .opacity))
            }
            ContentCard(model: model, settings: settings)
                .padding(.vertical, 8)
                .padding(.trailing, 8)
                .padding(.leading, settings.sidebarShown ? 0 : 8)
        }
        .overlay(alignment: .leading) {
            if !settings.sidebarShown { peek(palette) }
        }
        .background { palette.sidebar.opacity(palette.spec.sidebarOpacity) }
        .background(WindowChrome(trafficLights: settings.trafficLightsVisible, measured: $lights))
        .ignoresSafeArea()
        .environment(\.palette, palette)
        .animation(.smooth(duration: 0.3), value: settings.sidebarShown)
        .sheet(isPresented: $settings.showSettings) {
            SettingsView(model: model, settings: settings)
                .environment(\.palette, palette)
                .tint(palette.accent)
                // Opaque, so the chat behind never shows through.
                .presentationBackground(palette.content)
        }
        .onChange(of: settings.sidebarShown) { _, shown in if shown { peeking = false } }
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

    /// A hover strip on the left edge and, while peeking, the sidebar itself:
    /// flush with the window edge, full height, the same tint over a material,
    /// lying over the content, which keeps its layout underneath.
    @ViewBuilder
    private func peek(_ palette: Palette) -> some View {
        ZStack(alignment: .leading) {
            Color.clear
                .frame(width: 10)
                .frame(maxHeight: .infinity)
                .contentShape(.rect)
                .onHover { if $0 { setPeek(true) } }
                .accessibilityHidden(true)
            if peeking {
                Sidebar(model: model, settings: settings, peeking: true, lights: TrafficLights(maxX: nil, centerY: lights.centerY))
                    .frame(width: settings.sidebarWidth)
                    .frame(maxHeight: .infinity)
                    .background {
                        ZStack {
                            Rectangle().fill(.ultraThinMaterial)
                            palette.sidebar.opacity(min(1, palette.spec.sidebarOpacity + 0.2))
                        }
                    }
                    .overlay(alignment: .trailing) {
                        Rectangle().fill(palette.sidebarText.opacity(0.1)).frame(width: 0.5)
                    }
                    .shadow(color: .black.opacity(palette.spec.dark ? 0.35 : 0.15), radius: 14, x: 3)
                    .onHover { setPeek($0) }
                    .transition(.move(edge: .leading))
            }
        }
        .animation(.smooth(duration: 0.25), value: peeking)
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
                        width = min(max(base + value.translation.width, AppSettings.sidebarMin), AppSettings.sidebarMax)
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
    @ObservedObject var model: ChatModel
    @ObservedObject var settings: AppSettings
    /// Shown as Arc-style peek over the content; its toggle docks it.
    var peeking = false
    /// The window buttons, when they show on this sidebar.
    var lights = TrafficLights()
    @Environment(\.palette) private var palette

    /// Height of the top row; its centre sits on the window buttons' centre.
    private let row: CGFloat = 34

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            // Radian's top row: the toggle on the left (after the window
            // buttons when they show), new chat on the right, all on the
            // window buttons' centre line.
            HStack(spacing: 8) {
                if let maxX = lights.maxX { Color.clear.frame(width: max(0, maxX - 12 + 8)) }
                GlassCircle(symbol: "sidebar.left", help: peeking ? "Keep Sidebar Open (⌃⌘S)" : "Hide Sidebar (⌃⌘S)") { settings.sidebarShown = peeking }
                Spacer()
                GlassCircle(symbol: "square.and.pencil", help: "New Chat (⌘N)") { model.newChat() }
            }
            .frame(height: row)
            .padding(.top, max(0, lights.centerY - row / 2))

            SearchPill(query: $model.query) { settings.showSettings = true }

            VStack(spacing: 2) {
                SidebarRow(symbol: "plus", title: "New Chat", selected: false) { model.newChat() }
                Divider().opacity(0.5).padding(.vertical, 4)
                ScrollView {
                    LazyVStack(spacing: 2) {
                        ForEach(model.visibleChats) { chat in
                            SidebarRow(symbol: chat.messages.contains { $0.diagram != nil } ? "scribble.variable" : "bubble.left",
                                       title: chat.title, selected: chat.id == model.selection) { model.selection = chat.id }
                                .help(chat.updated.formatted(.relative(presentation: .named)))
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

            HStack {
                AstridMark(ui: model.config.ui, size: 18)
                    .foregroundStyle(palette.sidebarText)
                    .frame(width: 38, height: 38)
                    .glassEffect(.regular, in: .circle)
                    .accessibilityHidden(true)
                Spacer()
                GlassCircle(symbol: "gearshape", help: "Settings (⌘,)") { settings.showSettings = true }
            }
            .padding(.bottom, 12)
        }
        .padding(.horizontal, 12)
        .foregroundStyle(palette.sidebarText)
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
    var action: () -> Void
    @Environment(\.palette) private var palette
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: symbol)
                    .font(.system(size: 13))
                    .frame(width: 18)
                    .opacity(0.8)
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
    @Environment(\.palette) private var palette

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                if !settings.sidebarShown {
                    // The window buttons are hidden while the sidebar is closed.
                    GlassCircle(symbol: "sidebar.left", help: "Show Sidebar (⌃⌘S)") { settings.sidebarShown = true }
                    GlassCircle(symbol: "square.and.pencil", help: "New Chat (⌘N)") { model.newChat() }
                }
                Text(model.selected?.title ?? "New Chat")
                    .font(.headline)
                    .lineLimit(1)
                Spacer()
            }
            .padding(.horizontal, 16)
            .frame(height: 50)
            ThreadView(model: model)
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
            if thread.isEmpty { EmptyThread(ui: model.config.ui) }
        }
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

struct EmptyThread: View {
    var ui: URL
    @Environment(\.palette) private var palette

    var body: some View {
        VStack(spacing: 14) {
            AstridMark(ui: ui, size: 54)
                .foregroundStyle(palette.secondary)
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
    @FocusState private var focused: Bool
    @Environment(\.palette) private var palette

    private var running: Bool { model.live != nil }
    private var elsewhere: Bool { model.live.map { $0.chatID != model.selection } ?? false }

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
            GlassEffectContainer(spacing: 10) {
                HStack(alignment: .bottom, spacing: 10) {
                    TextField("Ask Astrid", text: $model.draft, axis: .vertical)
                        .textFieldStyle(.plain)
                        .lineLimit(1...8)
                        .focused($focused)
                        .onSubmit { model.send() }
                        .padding(.horizontal, 16)
                        .padding(.vertical, 11)
                        .glassEffect(.regular.interactive(), in: .rect(cornerRadius: 20))
                    if running {
                        Button { model.cancel() } label: {
                            Image(systemName: "stop.fill").frame(width: 22, height: 22)
                        }
                        .buttonStyle(.glass)
                        .buttonBorderShape(.circle)
                        .controlSize(.large)
                        .keyboardShortcut(.cancelAction)
                        .help("Stop (Esc)")
                    } else {
                        Button { model.send() } label: {
                            Image(systemName: "arrow.up").fontWeight(.semibold).frame(width: 22, height: 22)
                        }
                        .buttonStyle(.glassProminent)
                        .tint(palette.accent)
                        .buttonBorderShape(.circle)
                        .controlSize(.large)
                        .disabled(!model.canSend)
                        .help("Send (Return)")
                    }
                }
            }
        }
        .frame(maxWidth: 760)
        .padding(.horizontal, 28)
        .padding(.bottom, 16)
        .padding(.top, 6)
        .frame(maxWidth: .infinity)
        .onAppear { focused = true }
        .onChange(of: model.selection) { _, _ in focused = true }
    }
}
