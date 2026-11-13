import Combine
import DesktopCore
import Foundation

struct DesktopConfig {
    var ui: URL
    var child: AskChild.Config
    var chats: URL
}

/// Astrid's reply while it is being made: streamed text, the drawing so far,
/// the agent state and any approval the run is waiting on.
struct LiveReply {
    var id = UUID()
    var chatID: UUID
    var streamed = ""
    var label = "Thinking"
    var shapes: [JSONValue] = []
    var drawing = false
    var approval: (id: Int, title: String)?
    var answer: (text: String, brief: JSONValue?)?
    var error: String?

    /// What to show while streaming: the reply before its first fenced block
    /// (the canvas and brief blocks are drawn, not shown as text).
    var visibleText: String {
        let text = streamed.components(separatedBy: "```").first ?? ""
        return text.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

/// The app's state: chats from the store, the selected chat, and the one ask
/// that may be running. One ask runs at a time, through the app's own child.
final class ChatModel: ObservableObject {
    @Published private(set) var chats: [ChatSession]
    /// The selected chat; nil is a new chat that is saved once you send.
    @Published var selection: UUID?
    @Published var draft = ""
    @Published private(set) var live: LiveReply?
    @Published private(set) var saveError: String?
    /// Replies whose drawing should stream in rather than appear at once.
    private(set) var animated: Set<UUID> = []

    let config: DesktopConfig
    private let store: ChatStore
    private let child: AskChild

    init(config: DesktopConfig) {
        self.config = config
        store = ChatStore(directory: config.chats)
        child = AskChild(config: config.child)
        chats = store.loadAll()
        selection = chats.first?.id
        child.onEvent = { [weak self] event in self?.handle(event) }
    }

    var selected: ChatSession? { chats.first { $0.id == selection } }

    /// The selected chat's messages, with the reply in progress at the end.
    var thread: [ChatMessage] {
        var messages = selected?.messages ?? []
        if let live, live.chatID == selection {
            messages.append(ChatMessage(id: live.id, role: .astrid, text: live.answer?.text ?? live.visibleText, brief: live.answer?.brief, diagram: live.shapes.isEmpty ? nil : live.shapes))
        }
        return messages
    }

    var canSend: Bool {
        live == nil && !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    func newChat() {
        selection = nil
    }

    func send() {
        let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard live == nil, !text.isEmpty else { return }
        let message = ChatMessage(role: .user, text: String(text.prefix(4000)))
        var chat: ChatSession
        let history: [[String: String]]
        if let existing = selected {
            history = existing.history()
            chat = existing
            chat.append(message)
        } else {
            history = []
            chat = ChatSession(first: message)
        }
        draft = ""
        put(chat)
        selection = chat.id
        var reply = LiveReply(chatID: chat.id)
        animated.insert(reply.id)
        if !child.ask(message.text, history: history) {
            reply.error = "Could not start Astrid's runtime at \(config.child.astrid.path). Run `cargo build` in the repo, or pass --astrid."
            live = reply
            finish(outcome: "failed")
            return
        }
        live = reply
    }

    func cancel() {
        child.send(["type": "cancel"])
    }

    func answerApproval(_ allow: Bool) {
        guard let approval = live?.approval else { return }
        child.send(["type": "approve", "id": approval.id, "approve": allow])
        live?.approval = nil
    }

    func shutdown() {
        child.stop()
    }

    // MARK: Runtime events

    private func handle(_ event: AskEvent?) {
        guard live != nil else { return }
        switch event {
        case nil:
            live?.error = live?.error ?? "Astrid's runtime stopped. Ask again to restart it."
            finish(outcome: "failed")
        case .ready:
            break
        case .state(let state, let label):
            // The runtime goes idle after every ask, including ones that failed
            // before sending `done`.
            if state == "idle" { finish(outcome: live?.error == nil ? "completed" : "failed") } else { live?.label = label }
        case .delta(let text):
            live?.streamed += text
        case .approval(let id, let title):
            live?.approval = (id, title)
        case .canvasBegin:
            live?.shapes = []
            live?.drawing = true
        case .canvasShape(let shape):
            if (live?.shapes.count ?? 0) < ChatLimits.maxShapes { live?.shapes.append(shape) }
        case .canvasEnd:
            live?.drawing = false
        case .canvasCancel:
            live?.drawing = false
            live?.shapes = []
        case .answer(let text, let brief, _):
            live?.answer = (text, brief)
        case .error(let message):
            live?.error = message
        case .done(let outcome):
            finish(outcome: outcome)
        }
    }

    /// Saves the reply into its chat; once per ask.
    private func finish(outcome: String) {
        guard let reply = live else { return }
        live = nil
        guard var chat = chats.first(where: { $0.id == reply.chatID }) else { return }
        let text: String
        if let answer = reply.answer {
            text = answer.text
        } else if let error = reply.error {
            text = error
        } else if outcome == "cancelled" {
            text = reply.visibleText.isEmpty ? "Stopped." : reply.visibleText
        } else {
            text = reply.visibleText
        }
        chat.append(ChatMessage(id: reply.id, role: .astrid, text: text, brief: reply.answer?.brief, diagram: reply.shapes.isEmpty ? nil : reply.shapes, outcome: reply.error != nil && outcome == "completed" ? "failed" : outcome))
        put(chat)
    }

    /// Replaces or inserts a chat, keeps the list ordered and saves it.
    private func put(_ chat: ChatSession) {
        chats.removeAll { $0.id == chat.id }
        chats = ChatStore.sorted(chats + [chat])
        do {
            try store.save(chat)
            saveError = nil
        } catch {
            saveError = "Couldn't save this chat: \(error.localizedDescription)"
        }
    }
}
