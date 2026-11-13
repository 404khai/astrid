import AppKit
import DesktopCore
import SwiftUI

/// Sidebar of chats on the left, the selected chat on the right (ADR 0037).
/// The sidebar and toolbar take macOS 26's Liquid Glass from the system; the
/// composer and approval cards use `glassEffect` directly.
struct ContentView: View {
    @ObservedObject var model: ChatModel

    var body: some View {
        NavigationSplitView {
            Sidebar(model: model)
                .navigationSplitViewColumnWidth(min: 210, ideal: 250, max: 340)
        } detail: {
            ThreadView(model: model)
        }
        .onAppear {
            // The sidebar list can clear its selection as the window first
            // appears; open on the most recent chat, as the model intends.
            if !restored { model.selection = model.selection ?? model.chats.first?.id }
            restored = true
        }
    }

    @State private var restored = false
}

struct Sidebar: View {
    @ObservedObject var model: ChatModel

    var body: some View {
        List(selection: $model.selection) {
            ForEach(model.chats) { chat in
                VStack(alignment: .leading, spacing: 2) {
                    Text(chat.title)
                        .lineLimit(1)
                    Text(chat.updated, format: .relative(presentation: .named))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .padding(.vertical, 3)
                .tag(chat.id as UUID?)
            }
        }
        .overlay {
            if model.chats.isEmpty {
                ContentUnavailableView("No chats yet", systemImage: "bubble.left.and.text.bubble.right", description: Text("What you ask Astrid stays here."))
            }
        }
        .toolbar {
            ToolbarItem {
                Button { model.newChat() } label: {
                    Label("New Chat", systemImage: "square.and.pencil")
                }
                .help("New chat (⌘N)")
            }
        }
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
                .padding(.top, 24)
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
        .navigationTitle(model.selected?.title ?? "New Chat")
        .navigationSubtitle(model.selected.map { "\($0.messages.count) messages" } ?? "")
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

    var body: some View {
        VStack(spacing: 14) {
            AstridMark(ui: ui, size: 54)
                .foregroundStyle(.secondary)
            Text("Ask Astrid")
                .font(.title2.weight(.semibold))
            Text("Each chat keeps what you sent, Astrid's replies\nand the drawings she made.")
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
        }
        .padding(.bottom, 60)
    }
}

struct MessageRow: View {
    @ObservedObject var model: ChatModel
    var message: ChatMessage

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
                    .background(Color.accentColor.opacity(0.16), in: .rect(cornerRadius: 18))
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
                        .foregroundStyle(.secondary)
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
                    .foregroundStyle(.secondary)
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

    private func texts(_ key: String) -> [String] {
        brief[key]?.array?.compactMap { $0["text"]?.string } ?? []
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let title = brief["title"]?.string { Text(title).font(.headline) }
            if let summary = brief["summary"]?.string { Text(summary).foregroundStyle(.secondary) }
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
                .foregroundStyle(.secondary)
            }
        }
        .textSelection(.enabled)
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary.opacity(0.5), in: .rect(cornerRadius: 14))
    }
}

struct ApprovalCard: View {
    var title: String
    var decide: (Bool) -> Void

    var body: some View {
        GlassEffectContainer(spacing: 8) {
            HStack(spacing: 10) {
                Image(systemName: "envelope.badge.shield.half.filled")
                    .foregroundStyle(.secondary)
                Text(title).lineLimit(2)
                Spacer(minLength: 8)
                Button("Don't") { decide(false) }
                    .buttonStyle(.glass)
                Button("Allow once") { decide(true) }
                    .buttonStyle(.glassProminent)
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

    private var running: Bool { model.live != nil }
    private var elsewhere: Bool { model.live.map { $0.chatID != model.selection } ?? false }

    var body: some View {
        VStack(spacing: 6) {
            if elsewhere {
                Text("Astrid is answering in another chat.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
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
        .padding(.bottom, 18)
        .padding(.top, 6)
        .frame(maxWidth: .infinity)
        .onAppear { focused = true }
        .onChange(of: model.selection) { _, _ in focused = true }
    }
}
