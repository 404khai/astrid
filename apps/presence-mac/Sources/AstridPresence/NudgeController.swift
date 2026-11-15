import AppKit
import PresenceCore

/// ADR 0042: the resident host's nudges. Once a minute it reads the user's
/// choices (Settings → Nudges in the desktop app), what is coming up in the
/// calendar and any new important mail, and shows at most one short bubble
/// beside the presence mark when the interruption rule allows. A nudge never
/// acts on its own: only its yes button (opening a meeting link, or asking
/// Astrid exactly what the bubble said) does anything.
final class NudgeController {
    /// The calendar the nudges read: ADR 0041's EventKit source, set at
    /// launch unless `--ask-calendar off`.
    static var makeCalendar: () -> CalendarSource? = { nil }
    static let checkEvery: TimeInterval = 60
    static let mailEvery: TimeInterval = 10 * 60
    static let bubbleFor: TimeInterval = 120

    private weak var ask: AskController?
    private let astrid: URL
    private let settingsURL: URL
    private let stateURL: URL
    private var state: NudgeState
    private lazy var calendar: CalendarSource? = Self.makeCalendar()
    private var events: [CalendarEvent] = []
    private var eventsRead: Date?
    private var pendingMail: [MailHeader] = []
    private var mailChecked: Date?
    private var showing: (nudge: Nudge, at: Date)?
    private var working = false
    private var askedFocus = false
    private let work = DispatchQueue(label: "dev.astrid.presence.nudges")

    init(ask: AskController, astrid: URL, settingsURL: URL = NudgeSettings.defaultURL, stateURL: URL = NudgeState.defaultURL) {
        (self.ask, self.astrid, self.settingsURL, self.stateURL) = (ask, astrid, settingsURL, stateURL)
        state = NudgeState.load(from: stateURL)
    }

    /// Called from the frame loop; does real work at most once a minute.
    private var nextCheck = Date.distantPast
    func tick(now: Date = Date()) {
        if let showing, now.timeIntervalSince(showing.at) > Self.bubbleFor {
            // Unanswered counts as "Not now".
            ask?.clearNudge(id: showing.nudge.id)
            self.showing = nil
        }
        guard now >= nextCheck, !working else { return }
        nextCheck = now.addingTimeInterval(Self.checkEvery)
        // Unreadable settings mean everything is off.
        let settings = NudgeSettings.load(from: settingsURL) ?? NudgeSettings()
        guard settings.anyOn, showing == nil else { return }
        if !askedFocus {
            askedFocus = true
            SignalReader.requestFocusAccess()
        }
        let needEvents = settings.isOn(.morningBrief) || settings.isOn(.meetingSoon)
        let readEvents = needEvents && (eventsRead.map { now.timeIntervalSince($0) > 5 * 60 } ?? true)
        let mailSource = settings.isOn(.importantMail) ? settings.mailSource : nil
        let checkMail = mailSource != nil && (mailChecked.map { now.timeIntervalSince($0) > Self.mailEvery } ?? true)
        working = true
        let calendar = readEvents ? self.calendar : nil
        let astrid = self.astrid, senders = settings.importantSenders
        work.async { [weak self] in
            let day = Calendar.current.startOfDay(for: now)
            let events = calendar.flatMap { try? $0.events(from: day, to: now.addingTimeInterval(86400)) }
            let mail = checkMail ? Self.checkMail(astrid: astrid, source: mailSource!, senders: senders) : nil
            DispatchQueue.main.async { self?.decide(settings: settings, now: now, events: events, mail: mail.map { (mailSource!, $0) }) }
        }
    }

    private func decide(settings: NudgeSettings, now: Date, events: [CalendarEvent]?, mail: (String, [MailHeader])?) {
        working = false
        if let events {
            self.events = events
            eventsRead = now
        }
        if let (source, headers) = mail {
            mailChecked = now
            pendingMail += state.newMail(headers, source: source)
            pendingMail = Array(pendingMail.suffix(10))
            save()
        }
        guard let ask, showing == nil else { return }
        let candidates = NudgePlanner.candidates(settings: settings, now: now, events: self.events, mail: pendingMail, name: Self.name())
        let signals = SignalReader.current(busy: ask.busy, ownMic: ask.micActive)
        guard let (nudge, spoken) = Interruption.next(candidates, signals: signals, settings: settings, state: state, now: now) else { return }
        state.record(nudge, at: now)
        pendingMail.removeAll { "mail:\($0.id)" == nudge.id }
        save()
        showing = (nudge, now)
        ask.showNudge(nudge, spoken: spoken)
    }

    /// The bubble's buttons. Only "yes" on the nudge on screen does anything.
    func answer(id: String, choice: String) {
        guard let current = showing?.nudge, current.id == id else { return }
        showing = nil
        guard choice == "yes", let action = current.action else { return }
        switch action {
        case let .open(url):
            if MeetingLink.isMeeting(url) { NSWorkspace.shared.open(url) }
        case let .ask(text):
            ask?.askFromNudge(text)
        }
    }

    private func save() {
        try? state.save(to: stateURL)
    }

    /// The user's first name, if they told Astrid to remember it (ADR 0040).
    static func name() -> String? {
        let path = ProcessInfo.processInfo.environment["ASTRID_MEMORY_FILE"]
            ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support/Astrid/memory.json").path
        guard let data = FileManager.default.contents(atPath: path),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let facts = object["facts"] as? [[String: Any]] else { return nil }
        return NudgePlanner.name(fromMemory: facts.compactMap { $0["text"] as? String })
    }

    /// `astrid nudge-mail`: headers only, no model, 30 s at most.
    static func checkMail(astrid: URL, source: String, senders: [String]) -> [MailHeader]? {
        let process = Process()
        process.executableURL = astrid
        process.arguments = ["nudge-mail", "--source", source] + senders.prefix(10).flatMap { ["--sender", $0] }
        let output = Pipe()
        process.standardOutput = output
        process.standardError = FileHandle.nullDevice
        process.standardInput = FileHandle.nullDevice
        guard (try? process.run()) != nil else { return nil }
        let deadline = DispatchTime.now() + 30
        let done = DispatchSemaphore(value: 0)
        process.terminationHandler = { _ in done.signal() }
        if done.wait(timeout: deadline) == .timedOut {
            process.terminate()
            return nil
        }
        let data = output.fileHandleForReading.readDataToEndOfFile()
        guard data.count < 256 * 1024, let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let messages = object["messages"] as? [[String: Any]] else { return nil }
        return messages.prefix(20).compactMap { m in
            guard let id = m["id"] as? String else { return nil }
            return MailHeader(id: id, from: m["from"] as? String ?? "someone", subject: m["subject"] as? String ?? "(no subject)")
        }
    }
}
