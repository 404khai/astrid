import XCTest
@testable import PresenceCore

/// ADR 0042: the interruption rule and planner, on a fixed clock and zone.
final class NudgeTests: XCTestCase {
    let calendar: Calendar = {
        var c = Calendar(identifier: .gregorian)
        c.timeZone = TimeZone(identifier: "America/Los_Angeles")!
        return c
    }()

    func at(_ hhmm: String, day: Int = 16) -> Date {
        let parts = hhmm.split(separator: ":").map { Int($0)! }
        return calendar.date(from: DateComponents(year: 2026, month: 11, day: day, hour: parts[0], minute: parts[1]))!
    }

    func settings(_ kinds: NudgeKind...) -> NudgeSettings {
        var s = NudgeSettings()
        kinds.forEach { s.set($0, true) }
        return s
    }

    var review: CalendarEvent {
        CalendarEvent(id: "ek@1", title: "Design review", start: at("15:00"), end: at("16:00"),
                      meetingURL: URL(string: "https://acme.zoom.us/j/1"))
    }

    func testEveryKindIsOffUntilTurnedOnAndBadFilesMeanOff() throws {
        let off = NudgeSettings()
        XCTAssertFalse(off.anyOn)
        XCTAssertTrue(NudgePlanner.candidates(settings: off, now: at("14:52"), events: [review], mail: [], name: nil, calendar: calendar).isEmpty)
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let url = dir.appendingPathComponent("nudges.json")
        XCTAssertEqual(NudgeSettings.load(from: url), NudgeSettings(), "a missing file is the all-off default")
        var on = settings(.meetingSoon)
        on.meetingLeadMinutes = 500
        try on.save(to: url)
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        XCTAssertEqual((attributes[.posixPermissions] as? NSNumber)?.intValue, 0o600)
        XCTAssertEqual(NudgeSettings.load(from: url)?.meetingLeadMinutes, 30, "clamped")
        XCTAssertTrue(NudgeSettings.load(from: url)?.isOn(.meetingSoon) == true)
        try Data("{nope".utf8).write(to: url)
        XCTAssertNil(NudgeSettings.load(from: url), "unreadable: the host treats it as all off")
    }

    func testMeetingSoonOffersTheLinkAndNeverActsOnItsOwn() throws {
        let nudges = NudgePlanner.candidates(settings: settings(.meetingSoon), now: at("14:50"), events: [review], mail: [], name: nil, calendar: calendar)
        let nudge = try XCTUnwrap(nudges.first)
        XCTAssertEqual(nudge.text, "Design review starts in 10 minutes. Want me to open the link?")
        XCTAssertEqual(nudge.buttons.map(\.label), ["Open link", "Not now"])
        XCTAssertEqual(nudge.action, .open(URL(string: "https://acme.zoom.us/j/1")!))
        // Too early, or no link: nothing to open.
        XCTAssertTrue(NudgePlanner.candidates(settings: settings(.meetingSoon), now: at("14:40"), events: [review], mail: [], name: nil, calendar: calendar).isEmpty)
        var plain = review
        plain.meetingURL = URL(string: "https://example.com/meet")
        let info = NudgePlanner.candidates(settings: settings(.meetingSoon), now: at("14:55"), events: [plain], mail: [], name: nil, calendar: calendar)
        XCTAssertNil(info.first?.action)
        XCTAssertEqual(info.first?.buttons.map(\.label), ["OK"])
    }

    func testMorningBriefUsesTheNameFromMemoryAndOnlyRunsInTheMorning() throws {
        let name = NudgePlanner.name(fromMemory: ["I like short replies", "my name is KHAI"])
        XCTAssertEqual(name, "Khai")
        let brief = try XCTUnwrap(NudgePlanner.candidates(settings: settings(.morningBrief), now: at("08:45"), events: [review],
                                                          mail: [], name: name, calendar: calendar).first)
        XCTAssertEqual(brief.text, "Good morning, Khai. You have one thing today, starting with Design review at 3pm. Want the brief?")
        XCTAssertEqual(brief.action, .ask("What's my day?"))
        XCTAssertEqual(brief.id, "brief:2026-11-16")
        XCTAssertTrue(NudgePlanner.candidates(settings: settings(.morningBrief), now: at("08:10"), events: [], mail: [], name: nil, calendar: calendar).isEmpty)
        XCTAssertTrue(NudgePlanner.candidates(settings: settings(.morningBrief), now: at("12:30"), events: [], mail: [], name: nil, calendar: calendar).isEmpty)
    }

    func testImportantMailOnlyCountsMailThatArrivedAfterTheFirstCheck() {
        var state = NudgeState()
        let old = [MailHeader(id: "m1", from: "Ada", subject: "Old")]
        XCTAssertTrue(state.newMail(old, source: "gmail").isEmpty, "the first check only learns what's there")
        let fresh = state.newMail(old + [MailHeader(id: "m2", from: "Ada Lovelace", subject: "Contract signed")], source: "gmail")
        XCTAssertEqual(fresh.map(\.id), ["m2"])
        XCTAssertTrue(state.newMail(old, source: "gmail").isEmpty)
        let nudge = NudgePlanner.candidates(settings: settings(.importantMail), now: at("10:00"), events: [], mail: fresh, name: nil, calendar: calendar).first
        XCTAssertEqual(nudge?.text, "New email from Ada Lovelace: “Contract signed”. Want me to read it?")
        XCTAssertEqual(nudge?.action, .ask("Read the email from Ada Lovelace about “Contract signed” and tell me what it says."))
    }

    func testTheInterruptionRuleWaitsForEveryBusySignal() throws {
        let on = settings(.meetingSoon)
        let nudge = try XCTUnwrap(NudgePlanner.candidates(settings: on, now: at("14:52"), events: [review], mail: [], name: nil, calendar: calendar).first)
        let now = at("14:52"), state = NudgeState()
        func decide(_ signals: InterruptionSignals, _ settings: NudgeSettings = on, _ state: NudgeState = NudgeState(), now: Date? = nil) -> InterruptionDecision {
            Interruption.decide(nudge, signals: signals, settings: settings, state: state, now: now ?? at("14:52"), calendar: calendar)
        }
        XCTAssertEqual(decide(InterruptionSignals()), .show(spoken: false))
        XCTAssertEqual(decide(InterruptionSignals(focus: true)), .wait("focus"))
        XCTAssertEqual(decide(InterruptionSignals(screenShared: true)), .wait("screen_shared"))
        XCTAssertEqual(decide(InterruptionSignals(inCall: true)), .wait("in_call"))
        XCTAssertEqual(decide(InterruptionSignals(typing: true)), .wait("typing"))
        XCTAssertEqual(decide(InterruptionSignals(fullScreen: true)), .wait("full_screen"))
        XCTAssertEqual(decide(InterruptionSignals(busy: true)), .wait("busy"))
        XCTAssertEqual(decide(InterruptionSignals(locked: true)), .wait("locked"))
        XCTAssertEqual(decide(InterruptionSignals(), NudgeSettings()), .drop("off"))
        XCTAssertEqual(decide(InterruptionSignals(), now: at("15:03")), .drop("expired"))
        var shown = state
        shown.record(nudge, at: now)
        XCTAssertEqual(decide(InterruptionSignals(), on, shown), .drop("already_shown"))
        _ = state
    }

    func testSpeakingNeedsTheOptInAndAKnownFocusOff() throws {
        var on = settings(.meetingSoon)
        let nudge = try XCTUnwrap(NudgePlanner.candidates(settings: on, now: at("14:52"), events: [review], mail: [], name: nil, calendar: calendar).first)
        let decide = { (signals: InterruptionSignals, settings: NudgeSettings) in
            Interruption.decide(nudge, signals: signals, settings: settings, state: NudgeState(), now: self.at("14:52"), calendar: self.calendar)
        }
        XCTAssertEqual(decide(InterruptionSignals(), on), .show(spoken: false))
        on.speak = true
        XCTAssertEqual(decide(InterruptionSignals(focus: false), on), .show(spoken: true))
        XCTAssertEqual(decide(InterruptionSignals(focus: nil), on), .show(spoken: false), "unknown Focus: bubble only")
    }

    func testQuietHoursWrapMidnightAndRateLimitsHold() throws {
        let on = settings(.importantMail, .meetingSoon)
        XCTAssertTrue(Interruption.inQuietHours(at("22:30"), on, calendar: calendar))
        XCTAssertTrue(Interruption.inQuietHours(at("07:59"), on, calendar: calendar))
        XCTAssertFalse(Interruption.inQuietHours(at("08:00"), on, calendar: calendar))
        let mail = NudgePlanner.candidates(settings: on, now: at("22:30"), events: [], mail: [MailHeader(id: "m", from: "A", subject: "S")], name: nil, calendar: calendar)[0]
        XCTAssertEqual(Interruption.decide(mail, signals: InterruptionSignals(), settings: on, state: NudgeState(), now: at("22:30"), calendar: calendar), .wait("quiet_hours"))

        var state = NudgeState()
        let first = NudgePlanner.candidates(settings: on, now: at("10:00"), events: [], mail: [MailHeader(id: "a", from: "A", subject: "S")], name: nil, calendar: calendar)[0]
        state.record(first, at: at("10:00"))
        let second = NudgePlanner.candidates(settings: on, now: at("10:05"), events: [], mail: [MailHeader(id: "b", from: "B", subject: "T")], name: nil, calendar: calendar)[0]
        XCTAssertEqual(Interruption.decide(second, signals: InterruptionSignals(), settings: on, state: state, now: at("10:05"), calendar: calendar), .wait("rate_limit"))
        // A meeting about to start only needs a short gap.
        var meeting = review
        meeting.start = at("10:12")
        meeting.end = at("10:30")
        let soon = NudgePlanner.candidates(settings: on, now: at("10:05"), events: [meeting], mail: [], name: nil, calendar: calendar)[0]
        XCTAssertEqual(Interruption.decide(soon, signals: InterruptionSignals(), settings: on, state: state, now: at("10:05"), calendar: calendar), .show(spoken: false))
        // And it goes first when several are due.
        let picked = Interruption.next([second, soon], signals: InterruptionSignals(), settings: on, state: NudgeState(), now: at("10:05"), calendar: calendar)
        XCTAssertEqual(picked?.0.kind, .meetingSoon)
        for i in 0..<Interruption.perDay {
            state.record(Nudge(id: "x\(i)", kind: .importantMail, text: "", yes: nil, action: nil, notBefore: at("09:00"), expires: at("23:00")),
                         at: at("09:00").addingTimeInterval(TimeInterval(i)))
        }
        XCTAssertEqual(Interruption.decide(soon, signals: InterruptionSignals(), settings: on, state: state, now: at("10:05"), calendar: calendar), .wait("daily_limit"))
    }
}
