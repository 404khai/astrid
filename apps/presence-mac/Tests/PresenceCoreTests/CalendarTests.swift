import XCTest
@testable import PresenceCore

/// ADR 0041: a fake calendar, so no test touches EventKit or real data.
final class FakeCalendar: CalendarSource {
    var events: [CalendarEvent] = []
    var reminders: [ReminderItem] = []
    var denyEvents = false
    var denyReminders = false
    var hasHome = true
    var created: [(String, Date?, ReminderPlace?)] = []
    var moved: [(String, Date, Date)] = []
    var added: [(String, CalendarChoice)] = []
    var calendars = ["Work"]

    func events(from: Date, to: Date) throws -> [CalendarEvent] {
        if denyEvents { throw CalendarFailure.calendarAccessDenied }
        return events.filter { $0.end > from && $0.start < to }
    }

    func reminders(dueBefore: Date) throws -> [ReminderItem] {
        if denyReminders { throw CalendarFailure.remindersAccessDenied }
        return reminders
    }

    func createReminder(title: String, due: Date?, place: ReminderPlace?) throws -> ReminderItem {
        if place == .home, !hasHome { throw CalendarFailure.homeUnavailable }
        created.append((title, due, place))
        return ReminderItem(id: "r\(created.count)", title: title, due: due, place: place?.rawValue)
    }

    func createEvent(title: String, start: Date, end: Date, in choice: CalendarChoice) throws -> CalendarEvent {
        if case let .named(name) = choice, !calendars.contains(name) { throw CalendarFailure.calendarNotFound }
        added.append((title, choice))
        let account = choice == .apple ? "apple" : "google"
        return CalendarEvent(id: "n@1", title: title, start: start, end: end, calendar: choice == .apple ? "Home" : "me@gmail.com", account: account)
    }

    func moveEvent(id: String, expectStart: Date, start: Date, end: Date) throws -> CalendarEvent {
        guard let index = events.firstIndex(where: { $0.id == id }) else { throw CalendarFailure.eventNotFound }
        guard events[index].start == expectStart else { throw CalendarFailure.eventChanged }
        guard events[index].editable else { throw CalendarFailure.readOnlyCalendar }
        events[index].start = start
        events[index].end = end
        moved.append((id, start, end))
        return events[index]
    }
}

final class CalendarTests: XCTestCase {
    let now = CalendarTime.parse("2026-11-16T08:00:00Z")!
    func at(_ text: String) -> Date { CalendarTime.parse(text)! }

    func fake() -> FakeCalendar {
        let fake = FakeCalendar()
        fake.events = [
            CalendarEvent(id: "a@1", title: "Design review", start: at("2026-11-16T15:00:00Z"), end: at("2026-11-16T16:00:00Z"),
                          location: "Room 4", meetingURL: URL(string: "https://zoom.us/j/1")),
            CalendarEvent(id: "b@2", title: "Holiday", start: at("2026-11-16T00:00:00Z"), end: at("2026-11-17T00:00:00Z"),
                          allDay: true, calendar: "Holidays", editable: false),
        ]
        fake.reminders = [ReminderItem(id: "r0", title: "Call mum", due: at("2026-11-16T18:00:00Z"))]
        return fake
    }

    func testAgendaSendsTimesAndWhetherALinkExistsButNeverTheLinkOrNotes() throws {
        let service = CalendarService(source: fake())
        let reply = service.handle(["request_id": "q1", "op": "agenda", "from": "2026-11-16T00:00:00Z", "to": "2026-11-17T00:00:00Z"], now: now)
        XCTAssertEqual(reply["ok"] as? Bool, true)
        XCTAssertEqual(reply["request_id"] as? String, "q1")
        let data = try XCTUnwrap(reply["data"] as? [String: Any])
        let events = try XCTUnwrap(data["events"] as? [[String: Any]])
        XCTAssertEqual(events.map { $0["title"] as? String }, ["Holiday", "Design review"])
        let review = events[1]
        XCTAssertEqual(review["has_meeting_link"] as? Bool, true)
        XCTAssertEqual(review["location"] as? String, "Room 4")
        XCTAssertEqual(CalendarTime.parse(review["start"] as! String), at("2026-11-16T15:00:00Z"))
        XCTAssertNil(review["meeting_url"])
        XCTAssertNil(review["notes"])
        XCTAssertEqual(events[0]["editable"] as? Bool, false)
        XCTAssertEqual((data["reminders"] as? [[String: Any]])?.first?["title"] as? String, "Call mum")
    }

    func testAgendaIsBoundedAndDeniedAccessIsCategorical() {
        let source = fake()
        let service = CalendarService(source: source)
        let long = service.handle(["request_id": "q", "op": "agenda", "from": "2026-11-01T00:00:00Z", "to": "2026-11-30T00:00:00Z"], now: now)
        XCTAssertEqual(long["reason"] as? String, "invalid_request")
        let backwards = service.handle(["request_id": "q", "op": "agenda", "from": "2026-11-17T00:00:00Z", "to": "2026-11-16T00:00:00Z"], now: now)
        XCTAssertEqual(backwards["reason"] as? String, "invalid_request")
        source.denyReminders = true
        let partial = service.handle(["request_id": "q", "op": "agenda", "from": "2026-11-16T00:00:00Z", "to": "2026-11-17T00:00:00Z"], now: now)
        XCTAssertEqual(partial["ok"] as? Bool, true)
        XCTAssertEqual((partial["data"] as? [String: Any])?["reminders_unavailable"] as? String, "reminders_access_denied")
        source.denyEvents = true
        let denied = service.handle(["request_id": "q", "op": "agenda", "from": "2026-11-16T00:00:00Z", "to": "2026-11-17T00:00:00Z"], now: now)
        XCTAssertEqual(denied["ok"] as? Bool, false)
        XCTAssertEqual(denied["reason"] as? String, "calendar_access_denied")
    }

    func testRemindersAreValidatedAndHomeNeedsAnAddress() {
        let source = fake()
        let service = CalendarService(source: source)
        let timed = service.handle(["request_id": "w", "op": "reminder_create", "title": "  Buy milk ", "due": "2026-11-16T18:00:00Z", "place": NSNull()], now: now)
        XCTAssertEqual(timed["ok"] as? Bool, true)
        XCTAssertEqual(source.created.first?.0, "Buy milk")
        let home = service.handle(["request_id": "w", "op": "reminder_create", "title": "Water plants", "due": NSNull(), "place": "home"], now: now)
        XCTAssertEqual(((home["data"] as? [String: Any])?["reminder"] as? [String: Any])?["place"] as? String, "home")
        source.hasHome = false
        let missing = service.handle(["request_id": "w", "op": "reminder_create", "title": "Water plants", "place": "home"], now: now)
        XCTAssertEqual(missing["reason"] as? String, "home_address_unavailable")
        for bad: [String: Any] in [
            ["op": "reminder_create", "title": ""],
            ["op": "reminder_create", "title": String(repeating: "x", count: 201)],
            ["op": "reminder_create", "title": "a\u{7}b"],
            ["op": "reminder_create", "title": "x", "place": "work"],
            ["op": "reminder_create", "title": "x", "due": "2028-01-01T00:00:00Z"],
            ["op": "reminder_delete", "id": "r0"],
            ["op": "event_delete", "event_id": "a@1"],
        ] {
            XCTAssertEqual(service.handle(bad, now: now)["reason"] as? String, "invalid_request", "\(bad)")
        }
        XCTAssertEqual(source.created.count, 2)
    }

    func testMovingChecksTheEventStillStartsWhereTheUserWasAsked() {
        let source = fake()
        let service = CalendarService(source: source)
        let stale = service.handle(["request_id": "m", "op": "event_move", "event_id": "a@1", "expect_start": "2026-11-16T14:00:00Z",
                                    "start": "2026-11-16T16:00:00Z", "end": "2026-11-16T17:00:00Z"], now: now)
        XCTAssertEqual(stale["reason"] as? String, "event_changed")
        let readOnly = service.handle(["request_id": "m", "op": "event_move", "event_id": "b@2", "expect_start": "2026-11-16T00:00:00Z",
                                       "start": "2026-11-17T00:00:00Z", "end": "2026-11-17T12:00:00Z"], now: now)
        XCTAssertEqual(readOnly["reason"] as? String, "calendar_read_only")
        let tooLong = service.handle(["request_id": "m", "op": "event_move", "event_id": "a@1", "expect_start": "2026-11-16T15:00:00Z",
                                      "start": "2026-11-16T16:00:00Z", "end": "2026-11-18T17:00:00Z"], now: now)
        XCTAssertEqual(tooLong["reason"] as? String, "invalid_request")
        XCTAssertTrue(source.moved.isEmpty)
        let moved = service.handle(["request_id": "m", "op": "event_move", "event_id": "a@1", "expect_start": "2026-11-16T15:00:00Z",
                                    "start": "2026-11-16T16:00:00Z", "end": "2026-11-16T17:00:00Z"], now: now)
        XCTAssertEqual(moved["ok"] as? Bool, true)
        XCTAssertEqual(source.moved.first?.1, at("2026-11-16T16:00:00Z"))
    }

    func testMeetingLinksAreOnlyKnownHttpsHosts() {
        XCTAssertEqual(MeetingLink.find(in: [nil, "Join: https://acme.zoom.us/j/123?pwd=x thanks"])?.host, "acme.zoom.us")
        XCTAssertEqual(MeetingLink.find(in: ["https://meet.google.com/abc-defg-hij"])?.host, "meet.google.com")
        XCTAssertNil(MeetingLink.find(in: ["http://zoom.us/j/1", "https://evilzoom.us/j/1", "https://example.com"]))
    }

    func testCreatingDefaultsToGoogleAndLinksTheDayButNeverInvitesAnyone() throws {
        let source = fake()
        let service = CalendarService(source: source)
        let made = service.handle(["request_id": "c", "op": "event_create", "title": "Coffee chat", "calendar": NSNull(),
                                   "start": "2026-11-20T14:00:00-08:00", "end": "2026-11-20T17:00:00-08:00"], now: now)
        XCTAssertEqual(made["ok"] as? Bool, true)
        XCTAssertEqual(source.added.first?.1, .google)
        let event = try XCTUnwrap((made["data"] as? [String: Any])?["event"] as? [String: Any])
        XCTAssertEqual(event["account"] as? String, "google")
        XCTAssertTrue((event["web_url"] as? String)?.hasPrefix("https://calendar.google.com/calendar/r/day/2026/11/") == true)
        XCTAssertNil(event["attendees"])
        _ = service.handle(["request_id": "c", "op": "event_create", "title": "Gym", "calendar": "Apple",
                            "start": "2026-11-20T08:00:00-08:00", "end": "2026-11-20T09:00:00-08:00"], now: now)
        XCTAssertEqual(source.added.last?.1, .apple)
        let unknown = service.handle(["request_id": "c", "op": "event_create", "title": "X", "calendar": "Nope",
                                      "start": "2026-11-20T08:00:00-08:00", "end": "2026-11-20T09:00:00-08:00"], now: now)
        XCTAssertEqual(unknown["reason"] as? String, "calendar_not_found")
        let backwards = service.handle(["request_id": "c", "op": "event_create", "title": "X",
                                        "start": "2026-11-20T09:00:00-08:00", "end": "2026-11-20T08:00:00-08:00"], now: now)
        XCTAssertEqual(backwards["reason"] as? String, "invalid_request")
        XCTAssertEqual(CalendarChoice.parse("google calendar"), .google)
        XCTAssertEqual(CalendarChoice.parse("iCloud"), .apple)
        XCTAssertFalse(GoogleCalendarLink.isAllowed(URL(string: "https://calendar.google.com.evil.io/x")!))
        // Reads never carry the link.
        let agenda = service.handle(["request_id": "q", "op": "agenda", "from": "2026-11-16T00:00:00Z", "to": "2026-11-17T00:00:00Z"], now: now)
        XCTAssertNil(((agenda["data"] as? [String: Any])?["events"] as? [[String: Any]])?.first?["web_url"])
    }
}
