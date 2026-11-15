import Foundation

// ADR 0042: proactive nudges. Astrid may bring something up unasked (a
// morning brief, a meeting about to start, an important email) only when every
// rule here says so. This file is pure: settings, what could be said
// (`NudgePlanner`), and whether to say it now (`Interruption`). The resident
// host gathers the signals, owns the timer and shows the bubble.

public enum NudgeKind: String, Codable, CaseIterable, Sendable {
    case morningBrief = "morning_brief"
    case meetingSoon = "meeting_soon"
    case importantMail = "important_mail"
}

/// What the user chose in the desktop app's Settings → Nudges. Every kind is
/// off until turned on; an unreadable file means everything is off.
public struct NudgeSettings: Codable, Equatable, Sendable {
    public var version = 1
    public var enabled: [String: Bool] = [:]
    /// Say nudges out loud too (only when Focus is known to be off).
    public var speak = false
    /// Quiet hours, minutes after local midnight; start > end wraps midnight.
    public var quietStart = 21 * 60
    public var quietEnd = 8 * 60
    public var morningAt = 8 * 60 + 30
    public var meetingLeadMinutes = 10
    /// "gmail", "apple" or nil: where important mail is checked.
    public var mailSource: String?
    /// Senders whose new mail counts as important (names or addresses).
    public var importantSenders: [String] = []

    public init() {}

    public func isOn(_ kind: NudgeKind) -> Bool { enabled[kind.rawValue] == true }

    public mutating func set(_ kind: NudgeKind, _ on: Bool) { enabled[kind.rawValue] = on }

    public var anyOn: Bool { NudgeKind.allCases.contains(where: isOn) }

    public static var defaultURL: URL {
        if let path = ProcessInfo.processInfo.environment["ASTRID_NUDGES_FILE"] { return URL(fileURLWithPath: path) }
        return FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/Astrid/nudges.json")
    }

    /// Clamps every number into range so a hand-edited file can't widen anything.
    public func clamped() -> NudgeSettings {
        var s = self
        s.quietStart = min(max(s.quietStart, 0), 1439)
        s.quietEnd = min(max(s.quietEnd, 0), 1439)
        s.morningAt = min(max(s.morningAt, 5 * 60), 11 * 60 + 45)
        s.meetingLeadMinutes = min(max(s.meetingLeadMinutes, 1), 30)
        if !["gmail", "apple"].contains(s.mailSource ?? "") { s.mailSource = nil }
        s.importantSenders = Array(s.importantSenders.map { String($0.trimmingCharacters(in: .whitespaces).prefix(120)) }
            .filter { !$0.isEmpty }.prefix(20))
        return s
    }

    /// Missing file: defaults (all off). Unreadable or malformed: nil, and the
    /// host treats that as all off and never overwrites the file.
    public static func load(from url: URL = defaultURL) -> NudgeSettings? {
        guard let data = try? Data(contentsOf: url) else {
            return FileManager.default.fileExists(atPath: url.path) ? nil : NudgeSettings()
        }
        guard let settings = try? JSONDecoder().decode(NudgeSettings.self, from: data), settings.version == 1 else { return nil }
        return settings.clamped()
    }

    public func save(to url: URL = defaultURL) throws {
        try PrivateFile.write(try JSONEncoder.sorted.encode(clamped()), to: url)
    }
}

/// Atomic, user-only (0600) writes in a 0700 folder, like timers and memory.
public enum PrivateFile {
    public static func write(_ data: Data, to url: URL) throws {
        let folder = url.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let temp = folder.appendingPathComponent(".\(url.lastPathComponent).\(UUID().uuidString)")
        guard FileManager.default.createFile(atPath: temp.path, contents: data, attributes: [.posixPermissions: 0o600]) else {
            throw CocoaError(.fileWriteUnknown)
        }
        if rename(temp.path, url.path) != 0 {
            try? FileManager.default.removeItem(at: temp)
            throw CocoaError(.fileWriteUnknown)
        }
    }
}

extension JSONEncoder {
    static var sorted: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601
        return encoder
    }
}

/// A button on a nudge's bubble.
public struct NudgeButton: Equatable, Sendable {
    public var id: String
    public var label: String
    public init(_ id: String, _ label: String) { (self.id, self.label) = (id, label) }
}

/// What a yes does. Nothing happens without one.
public enum NudgeAction: Equatable, Sendable {
    /// Open a meeting link (only `MeetingLink` hosts) in the browser.
    case open(URL)
    /// Ask Astrid this, exactly as if the user had typed it.
    case ask(String)
}

public struct Nudge: Equatable, Sendable {
    /// Stable per thing: a nudge is shown at most once.
    public var id: String
    public var kind: NudgeKind
    public var text: String
    public var yes: NudgeButton?
    public var action: NudgeAction?
    public var notBefore: Date
    public var expires: Date

    public var buttons: [NudgeButton] {
        (yes.map { [$0] } ?? []) + [NudgeButton("no", yes == nil ? "OK" : "Not now")]
    }
}

/// One new message from the runtime's header-only check (`astrid nudge-mail`).
public struct MailHeader: Equatable, Sendable {
    public var id: String
    public var from: String
    public var subject: String
    public init(id: String, from: String, subject: String) { (self.id, self.from, self.subject) = (id, from, subject) }
}

public enum NudgePlanner {
    static func minutes(_ date: Date, _ calendar: Calendar) -> Int {
        let parts = calendar.dateComponents([.hour, .minute], from: date)
        return (parts.hour ?? 0) * 60 + (parts.minute ?? 0)
    }

    static func clock(_ date: Date, _ calendar: Calendar) -> String {
        let parts = calendar.dateComponents([.hour, .minute], from: date)
        let h = parts.hour ?? 0, m = parts.minute ?? 0
        if h == 12, m == 0 { return "noon" }
        let hour = h % 12 == 0 ? 12 : h % 12
        return m == 0 ? "\(hour)\(h < 12 ? "am" : "pm")" : String(format: "%d:%02d%@", hour, m, h < 12 ? "am" : "pm")
    }

    static func short(_ text: String, _ limit: Int = 60) -> String {
        let clean = text.replacingOccurrences(of: "\n", with: " ").trimmingCharacters(in: .whitespaces)
        return clean.count <= limit ? clean : String(clean.prefix(limit - 1)) + "…"
    }

    /// The user's first name from memory ("my name is Khai"), if they told Astrid.
    public static func name(fromMemory facts: [String]) -> String? {
        for fact in facts {
            let lower = fact.lowercased()
            guard let range = lower.range(of: "name is ") else { continue }
            let rest = fact[range.upperBound...].split(whereSeparator: { !$0.isLetter && $0 != "-" && $0 != "'" })
            if let first = rest.first, first.count <= 30 {
                return first.prefix(1).uppercased() + first.dropFirst().lowercased()
            }
        }
        return nil
    }

    /// Everything that could be brought up now. `Interruption` decides.
    public static func candidates(settings: NudgeSettings, now: Date, events: [CalendarEvent], mail: [MailHeader],
                                  name: String?, calendar: Calendar = .current) -> [Nudge] {
        var out: [Nudge] = []
        let day = calendar.startOfDay(for: now)
        if settings.isOn(.morningBrief) {
            let at = day.addingTimeInterval(TimeInterval(settings.morningAt * 60))
            let noon = day.addingTimeInterval(12 * 3600)
            if now >= at, now < noon {
                let today = events.filter { !$0.allDay && $0.end > now && calendar.isDate($0.start, inSameDayAs: now) }
                    .sorted { $0.start < $1.start }
                let hello = name.map { "Good morning, \($0)." } ?? "Good morning."
                let body: String
                if let first = today.first {
                    let count = today.count == 1 ? "one thing" : "\(today.count) things"
                    body = "You have \(count) today, starting with \(short(first.title, 40)) at \(clock(first.start, calendar))."
                } else {
                    body = "Your calendar is clear today."
                }
                let key = ISO8601DateFormatter.string(from: day, timeZone: calendar.timeZone, formatOptions: [.withFullDate])
                out.append(Nudge(id: "brief:\(key)", kind: .morningBrief, text: "\(hello) \(body) Want the brief?",
                                 yes: NudgeButton("yes", "Brief me"), action: .ask("What's my day?"), notBefore: at, expires: noon))
            }
        }
        if settings.isOn(.meetingSoon) {
            let lead = TimeInterval(settings.meetingLeadMinutes * 60)
            for event in events where !event.allDay && event.start > now && event.start <= now.addingTimeInterval(lead) {
                let minutes = max(1, Int((event.start.timeIntervalSince(now) / 60).rounded()))
                let when = minutes == 1 ? "in a minute" : "in \(minutes) minutes"
                let title = short(event.title, 50)
                if let url = event.meetingURL, MeetingLink.isMeeting(url) {
                    out.append(Nudge(id: "meeting:\(event.id)", kind: .meetingSoon,
                                     text: "\(title) starts \(when). Want me to open the link?",
                                     yes: NudgeButton("yes", "Open link"), action: .open(url),
                                     notBefore: event.start.addingTimeInterval(-lead), expires: event.start.addingTimeInterval(120)))
                } else {
                    out.append(Nudge(id: "meeting:\(event.id)", kind: .meetingSoon, text: "\(title) starts \(when).",
                                     yes: nil, action: nil,
                                     notBefore: event.start.addingTimeInterval(-lead), expires: event.start))
                }
            }
        }
        if settings.isOn(.importantMail) {
            for header in mail.prefix(3) {
                let from = short(header.from, 40), subject = short(header.subject, 60)
                out.append(Nudge(id: "mail:\(header.id)", kind: .importantMail,
                                 text: "New email from \(from): “\(subject)”. Want me to read it?",
                                 yes: NudgeButton("yes", "Read it"),
                                 action: .ask("Read the email from \(from) about “\(subject)” and tell me what it says."),
                                 notBefore: now, expires: now.addingTimeInterval(2 * 3600)))
            }
        }
        return out
    }
}

/// What the host can see right now. `nil` means it couldn't tell.
public struct InterruptionSignals: Equatable, Sendable {
    /// A Focus (including Do Not Disturb) is on.
    public var focus: Bool?
    public var screenShared = false
    /// Microphone or camera in use by another app.
    public var inCall = false
    /// A key was pressed in the last few seconds.
    public var typing = false
    /// The frontmost app fills a display (full-screen or presenting).
    public var fullScreen = false
    /// An ask, operator session or question is already on screen.
    public var busy = false
    public var locked = false

    public init(focus: Bool? = false, screenShared: Bool = false, inCall: Bool = false, typing: Bool = false,
                fullScreen: Bool = false, busy: Bool = false, locked: Bool = false) {
        (self.focus, self.screenShared, self.inCall, self.typing) = (focus, screenShared, inCall, typing)
        (self.fullScreen, self.busy, self.locked) = (fullScreen, busy, locked)
    }
}

/// When nudges were shown, and which mail was already seen. Kept for 2 days.
public struct NudgeState: Codable, Equatable, Sendable {
    public struct Shown: Codable, Equatable, Sendable {
        public var id: String
        public var kind: String
        public var at: Date
    }
    public var version = 1
    public var shown: [Shown] = []
    /// Message ids already known, per source; the first check only fills this.
    public var seenMail: [String: [String]] = [:]

    public init() {}

    public static var defaultURL: URL {
        FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support/Astrid/nudge-state.json")
    }

    public static func load(from url: URL) -> NudgeState {
        guard let data = try? Data(contentsOf: url) else { return NudgeState() }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return (try? decoder.decode(NudgeState.self, from: data)) ?? NudgeState()
    }

    public func save(to url: URL) throws {
        try PrivateFile.write(try JSONEncoder.sorted.encode(self), to: url)
    }

    public mutating func record(_ nudge: Nudge, at now: Date) {
        shown.append(Shown(id: nudge.id, kind: nudge.kind.rawValue, at: now))
        shown.removeAll { now.timeIntervalSince($0.at) > 2 * 86400 }
    }

    /// New mail ids for a source. The first call for a source returns none: it
    /// only learns what is already there, so turning this on never replays old mail.
    public mutating func newMail(_ headers: [MailHeader], source: String) -> [MailHeader] {
        let known = seenMail[source]
        let fresh = known.map { seen in headers.filter { !seen.contains($0.id) } } ?? []
        seenMail[source] = Array(((known ?? []) + headers.map(\.id)).suffix(500))
        return fresh
    }
}

public enum InterruptionDecision: Equatable, Sendable {
    case show(spoken: Bool)
    /// Not now; try again on a later check while the nudge is still current.
    case wait(String)
    /// Never show this one.
    case drop(String)
}

/// ADR 0042's interruption rule, in order. Conservative: when in doubt, wait.
public enum Interruption {
    /// At most one nudge per 20 minutes; a meeting reminder needs only 2 minutes.
    public static let gap: TimeInterval = 20 * 60
    public static let meetingGap: TimeInterval = 2 * 60
    public static let perDay = 8

    public static func inQuietHours(_ now: Date, _ settings: NudgeSettings, calendar: Calendar = .current) -> Bool {
        let m = NudgePlanner.minutes(now, calendar)
        let (start, end) = (settings.quietStart, settings.quietEnd)
        if start == end { return false }
        return start < end ? (m >= start && m < end) : (m >= start || m < end)
    }

    public static func decide(_ nudge: Nudge, signals: InterruptionSignals, settings: NudgeSettings, state: NudgeState,
                              now: Date, calendar: Calendar = .current) -> InterruptionDecision {
        guard settings.isOn(nudge.kind) else { return .drop("off") }
        guard now < nudge.expires else { return .drop("expired") }
        guard !state.shown.contains(where: { $0.id == nudge.id }) else { return .drop("already_shown") }
        guard now >= nudge.notBefore else { return .wait("not_yet") }
        if inQuietHours(now, settings, calendar: calendar) { return .wait("quiet_hours") }
        if signals.focus == true { return .wait("focus") }
        if signals.locked { return .wait("locked") }
        if signals.screenShared { return .wait("screen_shared") }
        if signals.inCall { return .wait("in_call") }
        if signals.fullScreen { return .wait("full_screen") }
        if signals.typing { return .wait("typing") }
        if signals.busy { return .wait("busy") }
        let recent = state.shown.filter { now.timeIntervalSince($0.at) < 86400 && calendar.isDate($0.at, inSameDayAs: now) }
        if recent.count >= perDay { return .wait("daily_limit") }
        let gap = nudge.kind == .meetingSoon ? meetingGap : gap
        if let last = state.shown.map(\.at).max(), now.timeIntervalSince(last) < gap { return .wait("rate_limit") }
        // Spoken only when asked for and Focus is known to be off.
        return .show(spoken: settings.speak && signals.focus == false)
    }

    /// The one nudge to show now, if any, with how. Meetings first, then mail, then the brief.
    public static func next(_ candidates: [Nudge], signals: InterruptionSignals, settings: NudgeSettings,
                            state: NudgeState, now: Date, calendar: Calendar = .current) -> (Nudge, spoken: Bool)? {
        let order: [NudgeKind] = [.meetingSoon, .importantMail, .morningBrief]
        for nudge in candidates.sorted(by: { order.firstIndex(of: $0.kind)! < order.firstIndex(of: $1.kind)! }) {
            if case let .show(spoken) = decide(nudge, signals: signals, settings: settings, state: state, now: now, calendar: calendar) {
                return (nudge, spoken)
            }
        }
        return nil
    }
}
