import Foundation

// ADR 0041: Calendar and Reminders through EventKit, owned by the presence
// host. This file is the surface shared by the ask path (`CalendarService`,
// answering `presence-ask`) and proactive nudges (ADR 0042), which read the
// same source in the host. EventKit itself lives in AstridPresence; tests use
// a fake `CalendarSource`.

/// One calendar event occurrence, as the host reads it.
public struct CalendarEvent: Equatable, Sendable {
    /// Opaque host id for this occurrence (EventKit id plus its start).
    public var id: String
    public var title: String
    public var start: Date
    public var end: Date
    public var allDay: Bool
    public var calendar: String
    public var location: String?
    /// A video-call link found in the event's URL, location or notes. Only
    /// known meeting hosts count (`MeetingLink`); notes are never sent on.
    public var meetingURL: URL?
    /// False for subscribed, birthday and other read-only calendars.
    public var editable: Bool

    public init(id: String, title: String, start: Date, end: Date, allDay: Bool = false, calendar: String = "Calendar",
                location: String? = nil, meetingURL: URL? = nil, editable: Bool = true) {
        (self.id, self.title, self.start, self.end, self.allDay, self.calendar) = (id, title, start, end, allDay, calendar)
        (self.location, self.meetingURL, self.editable) = (location, meetingURL, editable)
    }
}

/// One incomplete reminder.
public struct ReminderItem: Equatable, Sendable {
    public var id: String
    public var title: String
    public var due: Date?
    public var list: String
    /// "home" when it fires on arriving home.
    public var place: String?

    public init(id: String, title: String, due: Date? = nil, list: String = "Reminders", place: String? = nil) {
        (self.id, self.title, self.due, self.list, self.place) = (id, title, due, list, place)
    }
}

/// Where a location reminder fires. Only home, from the user's own contact card.
public enum ReminderPlace: String, Sendable { case home }

/// Categorical failures; the reason string is what the runtime sees.
public enum CalendarFailure: String, Error, Sendable {
    case calendarAccessDenied = "calendar_access_denied"
    case remindersAccessDenied = "reminders_access_denied"
    case eventNotFound = "event_not_found"
    case eventChanged = "event_changed"
    case readOnlyCalendar = "calendar_read_only"
    case homeUnavailable = "home_address_unavailable"
    case invalidRequest = "invalid_request"
    case failed = "calendar_failed"
}

/// What the host can do with the user's calendars and reminders. There is no
/// delete: ADR 0041 never removes events or reminders.
public protocol CalendarSource: AnyObject {
    func events(from: Date, to: Date) throws -> [CalendarEvent]
    func reminders(dueBefore: Date) throws -> [ReminderItem]
    func createReminder(title: String, due: Date?, place: ReminderPlace?) throws -> ReminderItem
    /// Moves one occurrence. Throws `eventChanged` unless it still starts at `expectStart`.
    func moveEvent(id: String, expectStart: Date, start: Date, end: Date) throws -> CalendarEvent
}

/// Finds a video-call link among an event's URL, location and notes.
public enum MeetingLink {
    public static let hosts = ["zoom.us", "meet.google.com", "teams.microsoft.com", "teams.live.com", "facetime.apple.com", "webex.com", "whereby.com", "around.co"]

    public static func isMeeting(_ url: URL) -> Bool {
        guard url.scheme == "https", let host = url.host?.lowercased() else { return false }
        return hosts.contains { host == $0 || host.hasSuffix("." + $0) }
    }

    public static func find(in texts: [String?]) -> URL? {
        let detector = try? NSDataDetector(types: NSTextCheckingResult.CheckingType.link.rawValue)
        for text in texts.compactMap({ $0 }) where !text.isEmpty {
            let range = NSRange(text.startIndex..., in: text)
            for match in detector?.matches(in: text, range: range) ?? [] {
                if let url = match.url, isMeeting(url) { return url }
            }
        }
        return nil
    }
}

/// ISO-8601 with the local offset, the wire format for every time.
public enum CalendarTime {
    public static func parse(_ text: String) -> Date? {
        let plain = ISO8601DateFormatter()
        plain.formatOptions = [.withInternetDateTime]
        if let date = plain.date(from: text) { return date }
        let fractional = ISO8601DateFormatter()
        fractional.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return fractional.date(from: text)
    }

    public static func format(_ date: Date, in zone: TimeZone = .current) -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        formatter.timeZone = zone
        return formatter.string(from: date)
    }
}
