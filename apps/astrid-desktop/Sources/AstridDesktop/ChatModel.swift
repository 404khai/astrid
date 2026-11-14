import Combine
import DesktopCore
import Foundation

struct DesktopConfig {
    var ui: URL
    var astrid: URL
    var chats: URL
    /// Launch flags that win over the stored settings.
    var modelOverride: String?
    var mailOverride: String?
}

/// Astrid's reply while it is being made: streamed text, the drawing so far,
/// the agent state and any approval the run is waiting on.
struct LiveReply {
    var id = UUID()
    var chatID: UUID
    /// Where the ask was typed: here, or on a paired phone (ADR 0039).
    var origin: ChatModel.Origin = .desktop
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
/// that may be running. One ask runs at a time, through the app's own child,
/// or for a paired phone through a second child with no mail source.
final class ChatModel: ObservableObject {
    enum Origin { case desktop, phone }

    /// What the phone link relays: chats as they are saved, and each event of
    /// the running ask, whichever side asked.
    enum Update {
        case saved(ChatSession)
        case live(chat: UUID, reply: UUID, event: AskEvent)
    }
    let updates = PassthroughSubject<Update, Never>()

    @Published private(set) var chats: [ChatSession]
    /// The selected chat; nil is a new chat that is saved once you send.
    @Published var selection: UUID?
    @Published var draft = ""
    /// Hands-free voice in the composer.
    let voice = VoiceMode()
    /// The reply being made for a voice ask, to speak when it's saved.
    private var voiceReply: UUID?
    /// The sidebar's search text.
    @Published var query = ""
    /// What the main content shows: a chat, or every saved drawing.
    @Published var page: Page = .chat
    /// The drawing open in the modal, if any.
    @Published var openDiagram: DiagramItem?

    enum Page { case chat, diagrams }
    @Published private(set) var live: LiveReply?
    @Published private(set) var saveError: String?
    /// Replies whose drawing should stream in rather than appear at once.
    private(set) var animated: Set<UUID> = []

    let config: DesktopConfig
    private let store: ChatStore
    private var child: AskChild
    private(set) var childConfig: AskChild.Config
    /// Phone asks get their own child without mail (ADR 0039): mail approvals
    /// are answered on the Mac, so a phone ask never reaches mail.
    private var phoneChild: AskChild

    init(config: DesktopConfig, model: String, mail: String) {
        self.config = config
        store = ChatStore(directory: config.chats)
        childConfig = AskChild.Config(astrid: config.astrid, model: model, mail: mail == "none" ? nil : mail)
        child = AskChild(config: childConfig)
        phoneChild = AskChild(config: AskChild.Config(astrid: config.astrid, model: model, mail: nil))
        chats = store.loadAll()
        selection = chats.first?.id
        child.onEvent = { [weak self] event in self?.handle(event, from: .desktop) }
        phoneChild.onEvent = { [weak self] event in self?.handle(event, from: .phone) }
        setUpVoice()
    }

    /// Chats matching the sidebar search.
    var visibleChats: [ChatSession] { ChatSearch.filter(chats, query: query) }

    /// Uses another model or mail source from the next ask on. Waits while an
    /// ask is running; the old child is stopped and a new one starts on demand.
    func reconfigure(model: String, mail: String) {
        let next = AskChild.Config(astrid: config.astrid, model: model, mail: mail == "none" ? nil : mail)
        guard live == nil, next.model != childConfig.model || next.mail != childConfig.mail else { return }
        child.stop()
        childConfig = next
        child = AskChild(config: next)
        child.onEvent = { [weak self] event in self?.handle(event, from: .desktop) }
        phoneChild.stop()
        phoneChild = AskChild(config: AskChild.Config(astrid: config.astrid, model: model, mail: nil))
        phoneChild.onEvent = { [weak self] event in self?.handle(event, from: .phone) }
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
        page = .chat
    }

    func show(chat id: UUID) {
        selection = id
        page = .chat
    }

    /// Every saved drawing, newest first.
    var diagrams: [DiagramItem] { ChatGallery.diagrams(in: chats) }

    /// Wires voice mode's asks into the selected chat.
    func setUpVoice() {
        voice.ask = { [weak self] text in
            guard let self, let started = self.ask(text, in: self.selected?.id, from: .desktop) else { return false }
            self.selection = started.chat
            self.voiceReply = started.reply
            return true
        }
        voice.stopAsk = { [weak self] in self?.cancel() }
    }

    /// Renames a chat or changes its icon or colour; false if nothing valid changed.
    @discardableResult
    func restyle(_ id: UUID, title: String? = nil, icon: String? = nil, color: String? = nil) -> Bool {
        guard let chat = chats.first(where: { $0.id == id }),
              let next = ChatStyle.apply(to: chat, title: title, icon: icon, color: color) else { return false }
        put(next)
        return true
    }

    func send() {
        let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard live == nil, !text.isEmpty else { return }
        draft = ""
        if let started = ask(text, in: selected?.id, from: .desktop) { selection = started.chat }
    }

    /// The running ask, if any: its chat and reply.
    var running: (chat: UUID, reply: UUID)? { live.map { ($0.chatID, $0.id) } }

    /// Starts an ask in a chat (nil for a new one) and returns where its reply
    /// goes; nil if an ask is already running or the chat is gone.
    @discardableResult
    func ask(_ text: String, in chatID: UUID?, from origin: Origin) -> (chat: UUID, reply: UUID)? {
        let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard live == nil, !text.isEmpty else { return nil }
        let message = ChatMessage(role: .user, text: String(text.prefix(4000)))
        var chat: ChatSession
        let history: [[String: String]]
        if let chatID {
            guard let existing = chats.first(where: { $0.id == chatID }) else { return nil }
            history = existing.history()
            chat = existing
            chat.append(message)
        } else {
            history = []
            chat = ChatSession(first: message)
        }
        put(chat)
        var reply = LiveReply(chatID: chat.id, origin: origin)
        animated.insert(reply.id)
        if !(origin == .phone ? phoneChild : child).ask(message.text, history: history) {
            reply.error = "Could not start Astrid's runtime at \(config.astrid.path). Run `cargo build` in the repo, or pass --astrid."
            live = reply
            finish(outcome: "failed")
            return (chat.id, reply.id)
        }
        live = reply
        return (chat.id, reply.id)
    }

    func cancel() {
        (live?.origin == .phone ? phoneChild : child).send(["type": "cancel"])
    }

    func answerApproval(_ allow: Bool) {
        guard let approval = live?.approval else { return }
        child.send(["type": "approve", "id": approval.id, "approve": allow])
        live?.approval = nil
    }

    func shutdown() {
        child.stop()
        phoneChild.stop()
    }

    // MARK: Runtime events

    private func handle(_ event: AskEvent?, from origin: Origin) {
        guard let current = live, current.origin == origin else { return }
        if let event { updates.send(.live(chat: current.chatID, reply: current.id, event: event)) }
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
        let message = ChatMessage(id: reply.id, role: .astrid, text: text, brief: reply.answer?.brief, diagram: reply.shapes.isEmpty ? nil : reply.shapes, outcome: reply.error != nil && outcome == "completed" ? "failed" : outcome)
        chat.append(message)
        put(chat)
        if voiceReply == reply.id {
            voiceReply = nil
            voice.replied(message)
        }
    }

    /// Replaces or inserts a chat, keeps the list ordered and saves it.
    private func put(_ chat: ChatSession) {
        chats.removeAll { $0.id == chat.id }
        chats = ChatStore.sorted(chats + [chat])
        updates.send(.saved(chat))
        do {
            try store.save(chat)
            saveError = nil
        } catch {
            saveError = "Couldn't save this chat: \(error.localizedDescription)"
        }
    }
}
