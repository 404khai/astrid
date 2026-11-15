import Foundation

/// ADR 0041: answers `presence-ask`'s `calendar` requests from a
/// `CalendarSource`, validating each one again in the host. The runtime has
/// already had the user confirm a write; this layer still bounds what a
/// confirmed request may do.
///
/// in:  {"type":"calendar","request_id","op":"agenda","from","to"}
///      | {"op":"reminder_create","title","due"|null,"place":"home"|null}
///      | {"op":"event_move","event_id","expect_start","start","end"}
/// out: {"type":"calendar_result","request_id","ok":true,"data":{..}}
///      | {"type":"calendar_result","request_id","ok":false,"reason":..}
public struct CalendarService {
    public static let maxRangeDays = 14.0
    public static let maxEvents = 50
    public static let maxReminders = 50
    public static let maxTitle = 200
    public static let maxText = 200
    /// Writes may land at most a year ahead and at most a day in the past.
    public static let maxAhead: TimeInterval = 366 * 86400
    public static let maxBehind: TimeInterval = 86400
    public static let maxEventLength: TimeInterval = 24 * 3600

    public let source: CalendarSource

    public init(source: CalendarSource) {
        self.source = source
    }

    public func handle(_ request: [String: Any], now: Date = Date()) -> [String: Any] {
        let id = request["request_id"] as? String ?? ""
        do {
            let data = try run(request, now: now)
            return ["type": "calendar_result", "request_id": id, "ok": true, "data": data]
        } catch let failure as CalendarFailure {
            return ["type": "calendar_result", "request_id": id, "ok": false, "reason": failure.rawValue]
        } catch {
            return ["type": "calendar_result", "request_id": id, "ok": false, "reason": CalendarFailure.failed.rawValue]
        }
    }

    private func time(_ value: Any?) throws -> Date {
        guard let text = value as? String, let date = CalendarTime.parse(text) else { throw CalendarFailure.invalidRequest }
        return date
    }

    private func run(_ request: [String: Any], now: Date) throws -> [String: Any] {
        switch request["op"] as? String {
        case "agenda":
            let from = try time(request["from"]), to = try time(request["to"])
            guard to > from, to.timeIntervalSince(from) <= Self.maxRangeDays * 86400 else { throw CalendarFailure.invalidRequest }
            let events = try source.events(from: from, to: to).sorted { $0.start < $1.start }
            // Reminders are optional: a denied Reminders grant still answers with events.
            let reminders = try? source.reminders(dueBefore: to)
            var data: [String: Any] = [
                "events": events.prefix(Self.maxEvents).map(Self.eventJSON),
                "truncated": events.count > Self.maxEvents || (reminders?.count ?? 0) > Self.maxReminders,
            ]
            if let reminders {
                data["reminders"] = reminders.prefix(Self.maxReminders).map(Self.reminderJSON)
            } else {
                data["reminders_unavailable"] = CalendarFailure.remindersAccessDenied.rawValue
            }
            return data
        case "reminder_create":
            guard let raw = request["title"] as? String else { throw CalendarFailure.invalidRequest }
            let title = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !title.isEmpty, title.count <= Self.maxTitle, Self.printable(title) else { throw CalendarFailure.invalidRequest }
            var due: Date?
            if let value = request["due"], !(value is NSNull) {
                let date = try time(value)
                guard date > now.addingTimeInterval(-Self.maxBehind), date < now.addingTimeInterval(Self.maxAhead) else { throw CalendarFailure.invalidRequest }
                due = date
            }
            var place: ReminderPlace?
            if let value = request["place"], !(value is NSNull) {
                guard let text = value as? String, let parsed = ReminderPlace(rawValue: text) else { throw CalendarFailure.invalidRequest }
                place = parsed
            }
            return ["reminder": Self.reminderJSON(try source.createReminder(title: title, due: due, place: place))]
        case "event_move":
            guard let id = request["event_id"] as? String, !id.isEmpty, id.count <= 512 else { throw CalendarFailure.invalidRequest }
            let expect = try time(request["expect_start"]), start = try time(request["start"]), end = try time(request["end"])
            guard end > start, end.timeIntervalSince(start) <= Self.maxEventLength,
                  start > now.addingTimeInterval(-Self.maxBehind), start < now.addingTimeInterval(Self.maxAhead) else {
                throw CalendarFailure.invalidRequest
            }
            return ["event": Self.eventJSON(try source.moveEvent(id: id, expectStart: expect, start: start, end: end))]
        default:
            throw CalendarFailure.invalidRequest
        }
    }

    static func printable(_ text: String) -> Bool {
        !text.unicodeScalars.contains { CharacterSet.controlCharacters.contains($0) }
    }

    static func clip(_ text: String?) -> Any {
        guard let text, !text.isEmpty else { return NSNull() }
        return String(text.prefix(maxText))
    }

    /// What the runtime gets for one event. Notes, attendees and the link
    /// itself stay in the host; only whether a meeting link exists goes on.
    public static func eventJSON(_ event: CalendarEvent) -> [String: Any] {
        [
            "id": event.id,
            "title": String(event.title.prefix(maxTitle)),
            "start": CalendarTime.format(event.start),
            "end": CalendarTime.format(event.end),
            "all_day": event.allDay,
            "calendar": String(event.calendar.prefix(80)),
            "location": clip(event.location),
            "has_meeting_link": event.meetingURL != nil,
            "editable": event.editable,
        ]
    }

    public static func reminderJSON(_ reminder: ReminderItem) -> [String: Any] {
        [
            "id": reminder.id,
            "title": String(reminder.title.prefix(maxTitle)),
            "due": reminder.due.map { CalendarTime.format($0) } ?? NSNull(),
            "list": String(reminder.list.prefix(80)),
            "place": reminder.place ?? NSNull(),
        ]
    }
}
