import Combine
import DesktopCore
import Foundation

/// Goblin group chats come alive (ADR 0037 amendment 14). Each group keeps a
/// thread; a message from the user starts a round in which whoever it tags
/// answers (Astrid, the overseer, if nobody is tagged), and anyone a reply
/// tags answers after it, one at a time, bounded by `GroupTurnPlan`. Every
/// turn is one ask to the app's own `astrid presence-ask` child for groups,
/// with no mail source; any approval it asks for is declined.
final class GroupChatModel: ObservableObject {
    /// What Astrid's eyes in the group are doing.
    enum Overseer: String { case idle, tagged, question, checking, talking }

    /// The turn in progress: who is speaking and what they've said so far.
    struct Live: Equatable {
        var group: UUID
        var speaker: String
        var streamed = ""
        var answer: String?
        var error: String?
        /// The message that handed this speaker the turn.
        var handoff: UUID?
    }

    @Published private(set) var threads: [UUID: GroupThread] = [:]
    @Published private(set) var live: Live?
    @Published private(set) var overseer: [UUID: Overseer] = [:]
    @Published var drafts: [UUID: String] = [:]
    @Published private(set) var memories: [String: GoblinMemory] = [:]
    @Published private(set) var problem: String?
    /// Bumped to open a group's side panel from the native settings button.
    @Published var panelRequest = 0

    private let store: GroupThreadStore
    private let memoryStore: GoblinMemoryStore
    private var child: AskChild
    private var config: AskChild.Config
    private var plan = GroupTurnPlan()
    private var round: (group: GoblinGroup, members: [GroupMember])?
    private var calm: [UUID: DispatchWorkItem] = [:]

    init(astrid: URL, model: String, root: URL) {
        store = GroupThreadStore(directory: root.appendingPathComponent("Groups", isDirectory: true))
        memoryStore = GoblinMemoryStore(directory: root.appendingPathComponent("Goblins/Memory", isDirectory: true))
        config = AskChild.Config(astrid: astrid, model: model, mail: nil)
        child = AskChild(config: config)
        child.onEvent = { [weak self] in self?.handle($0) }
    }

    func reconfigure(model: String) {
        guard live == nil, model != config.model else { return }
        child.stop()
        config.model = model
        child = AskChild(config: config)
        child.onEvent = { [weak self] in self?.handle($0) }
    }

    func shutdown() { child.stop() }

    func thread(_ group: UUID) -> GroupThread {
        if let thread = threads[group] { return thread }
        let loaded = store.load(group)
        DispatchQueue.main.async { if self.threads[group] == nil { self.threads[group] = loaded } }
        return loaded
    }

    func memory(_ goblin: String) -> GoblinMemory {
        if let memory = memories[goblin] { return memory }
        let loaded = memoryStore.load(goblin)
        DispatchQueue.main.async { if self.memories[goblin] == nil { self.memories[goblin] = loaded } }
        return loaded
    }

    func busy(_ group: UUID) -> Bool { live?.group == group }

    // MARK: Sending

    /// The user's message: saved, then a round of replies starts.
    func send(in group: GoblinGroup, members: [GroupMember]) {
        let text = (drafts[group.id] ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        guard live == nil, !text.isEmpty else { return }
        drafts[group.id] = ""
        let mentions = Mentions.find(in: text, members: members)
        append(GroupMessage(author: .you, text: text, mentions: mentions), to: group.id)
        look(group.id, mentions.contains("astrid") ? .tagged : text.hasSuffix("?") ? .question : .checking, hold: 1.6)
        round = (group, members)
        plan.start(mentions: mentions)
        nextTurn()
    }

    func stop() {
        plan.stop()
        child.send(["type": "cancel"])
    }

    /// The user's reaction on a message (toggled).
    func react(_ emoji: String, to message: UUID, in group: UUID) {
        update(group) { thread in
            guard let i = thread.messages.firstIndex(where: { $0.id == message }) else { return }
            thread.messages[i].toggle(emoji, by: .you)
        }
    }

    func clearMemory(_ goblin: String) {
        let empty = GoblinMemory()
        memories[goblin] = empty
        try? memoryStore.save(empty, for: goblin)
    }

    /// Keeps a role in the goblin's memory too, so it carries to other groups.
    func noteRole(_ role: String, for goblin: String, in group: String) {
        guard !role.isEmpty else { return }
        var memory = self.memory(goblin)
        memory.remember("In \(group) my role is: \(role)")
        memories[goblin] = memory
        try? memoryStore.save(memory, for: goblin)
    }

    func forget(_ group: UUID) {
        threads[group] = nil
        store.delete(group)
    }

    // MARK: Turns

    private func nextTurn() {
        guard let round, let speaker = plan.next() else {
            if let id = round?.group.id { look(id, .idle) }
            round = nil
            return
        }
        let thread = self.thread(round.group.id)
        let handoff = thread.messages.last { $0.mentions.contains(speaker) && $0.author != speaker }
        let prompt = GroupPrompt.build(group: round.group, speaker: speaker, members: round.members, thread: thread.messages, memory: speaker == "astrid" ? GoblinMemory() : memory(speaker), handoff: handoff?.author == "you" ? nil : handoff)
        live = Live(group: round.group.id, speaker: speaker, handoff: thread.messages.last?.id)
        look(round.group.id, .checking)
        if !child.ask(prompt, history: []) {
            live?.error = "Could not start Astrid's runtime at \(config.astrid.path). Run `cargo build` in the repo."
            finishTurn(outcome: "failed")
        }
    }

    private func handle(_ event: AskEvent?) {
        guard live != nil else { return }
        switch event {
        case nil:
            live?.error = live?.error ?? "Astrid's runtime stopped."
            finishTurn(outcome: "failed")
        case .delta(let text):
            live?.streamed += text
            if live?.speaker == "astrid", let id = live?.group { look(id, .talking) }
        case .approval(let id, _):
            // Group turns only talk: any tool step is declined.
            child.send(["type": "approve", "id": id, "approve": false])
        case .answer(let text, _, _):
            live?.answer = text
        case .error(let message):
            live?.error = message
        case .done(let outcome):
            finishTurn(outcome: outcome)
        case .state(let state, _):
            if state == "idle" { finishTurn(outcome: live?.error == nil ? "completed" : "failed") }
        default: break
        }
    }

    private func finishTurn(outcome: String) {
        guard let turn = live, let round else { live = nil; return }
        live = nil
        let reply = GroupReply.parse(turn.answer ?? turn.streamed)
        var text = reply.text
        if text.isEmpty { text = turn.error ?? (outcome == "cancelled" ? "Stopped." : "…") }
        let mentions = Mentions.find(in: text, members: round.members).filter { $0 != turn.speaker }
        let author = GroupAuthor(key: turn.speaker)
        update(round.group.id) { thread in
            if let emoji = reply.react, let target = turn.handoff, let i = thread.messages.firstIndex(where: { $0.id == target }) {
                thread.messages[i].toggle(emoji, by: author)
            }
            thread.append(GroupMessage(author: author, text: text, mood: reply.mood ?? (turn.error == nil ? "talking" : "sad"), mentions: mentions, outcome: turn.error != nil ? "failed" : outcome == "completed" ? nil : outcome, chart: reply.chart))
        }
        if let fact = reply.remember, turn.speaker != "astrid" {
            var memory = self.memory(turn.speaker)
            memory.remember(fact)
            memories[turn.speaker] = memory
            try? memoryStore.save(memory, for: turn.speaker)
        }
        guard outcome == "completed", turn.error == nil else {
            plan.stop()
            nextTurn()
            return
        }
        if mentions.contains("astrid") { look(round.group.id, .tagged, hold: 1.2) }
        plan.replied(turn.speaker, mentions: mentions)
        nextTurn()
    }

    // MARK: Helpers

    private func append(_ message: GroupMessage, to group: UUID) {
        update(group) { $0.append(message) }
    }

    private func update(_ group: UUID, _ change: (inout GroupThread) -> Void) {
        var thread = self.thread(group)
        change(&thread)
        threads[group] = thread
        do {
            try store.save(thread)
            problem = nil
        } catch {
            problem = "Couldn't save this group: \(error.localizedDescription)"
        }
    }

    /// Sets Astrid's eyes; with `hold`, a brief look that then settles on
    /// checking while a round runs, or idle.
    private func look(_ group: UUID, _ state: Overseer, hold: TimeInterval? = nil) {
        calm[group]?.cancel()
        overseer[group] = state
        guard let hold else { return }
        let settle = DispatchWorkItem { [weak self] in
            guard let self else { return }
            self.overseer[group] = self.live?.group == group ? (self.live?.speaker == "astrid" && !(self.live?.streamed.isEmpty ?? true) ? .talking : .checking) : .idle
        }
        calm[group] = settle
        DispatchQueue.main.asyncAfter(deadline: .now() + hold, execute: settle)
    }
}
