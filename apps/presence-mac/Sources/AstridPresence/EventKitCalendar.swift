import Contacts
import CoreLocation
import EventKit
import Foundation
import PresenceCore

/// ADR 0041: the user's calendars and reminders through EventKit. macOS asks
/// for Calendars and Reminders access on first use (full access: EventKit has
/// no read-only grant for reading). Every call blocks, so callers run it off
/// the main thread. Nothing here deletes anything.
final class EventKitCalendar: CalendarSource {
    private let store = EKEventStore()
    private var eventsGranted: Bool?
    private var remindersGranted: Bool?
    private let lock = NSLock()
    /// Home's coordinate, resolved once per launch from the user's own card.
    private var home: CLLocation?

    private func wait<T>(_ body: (@escaping (T) -> Void) -> Void) -> T? {
        let semaphore = DispatchSemaphore(value: 0)
        var result: T?
        body { value in
            result = value
            semaphore.signal()
        }
        // The first-use prompt waits for the user; give them two minutes.
        return semaphore.wait(timeout: .now() + 120) == .success ? result : nil
    }

    private func access(_ type: EKEntityType) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        if type == .event, let granted = eventsGranted { return granted }
        if type == .reminder, let granted = remindersGranted { return granted }
        let status = EKEventStore.authorizationStatus(for: type)
        var granted: Bool
        switch status {
        case .fullAccess: granted = true
        case .notDetermined:
            granted = wait { done in
                if type == .event {
                    store.requestFullAccessToEvents { ok, _ in done(ok) }
                } else {
                    store.requestFullAccessToReminders { ok, _ in done(ok) }
                }
            } ?? false
        default: granted = false
        }
        if type == .event { eventsGranted = granted } else { remindersGranted = granted }
        return granted
    }

    static func hostId(_ event: EKEvent) -> String {
        "\(event.eventIdentifier ?? event.calendarItemIdentifier)@\(Int(event.startDate.timeIntervalSince1970))"
    }

    private func convert(_ event: EKEvent) -> CalendarEvent {
        CalendarEvent(
            id: Self.hostId(event), title: event.title ?? "Untitled", start: event.startDate, end: event.endDate,
            allDay: event.isAllDay, calendar: event.calendar?.title ?? "Calendar", location: event.location,
            meetingURL: MeetingLink.find(in: [event.url?.absoluteString, event.location, event.notes]),
            editable: event.calendar?.allowsContentModifications ?? false)
    }

    func events(from: Date, to: Date) throws -> [CalendarEvent] {
        guard access(.event) else { throw CalendarFailure.calendarAccessDenied }
        let predicate = store.predicateForEvents(withStart: from, end: to, calendars: nil)
        return store.events(matching: predicate).map(convert)
    }

    func reminders(dueBefore: Date) throws -> [ReminderItem] {
        guard access(.reminder) else { throw CalendarFailure.remindersAccessDenied }
        let predicate = store.predicateForIncompleteReminders(withDueDateStarting: nil, ending: dueBefore, calendars: nil)
        let found: [EKReminder]? = wait { done in _ = store.fetchReminders(matching: predicate) { done($0 ?? []) } }
        return (found ?? []).map { reminder in
            ReminderItem(id: reminder.calendarItemIdentifier, title: reminder.title ?? "Reminder",
                         due: reminder.dueDateComponents.flatMap { Calendar.current.date(from: $0) },
                         list: reminder.calendar?.title ?? "Reminders",
                         place: reminder.alarms?.contains { $0.structuredLocation != nil } == true ? "home" : nil)
        }
    }

    func createReminder(title: String, due: Date?, place: ReminderPlace?) throws -> ReminderItem {
        guard access(.reminder) else { throw CalendarFailure.remindersAccessDenied }
        let reminder = EKReminder(eventStore: store)
        reminder.title = title
        guard let list = store.defaultCalendarForNewReminders() else { throw CalendarFailure.failed }
        reminder.calendar = list
        if let due {
            reminder.dueDateComponents = Calendar.current.dateComponents([.year, .month, .day, .hour, .minute, .timeZone], from: due)
            reminder.addAlarm(EKAlarm(absoluteDate: due))
        }
        if place == .home {
            let location = EKStructuredLocation(title: "Home")
            location.geoLocation = try homeLocation()
            location.radius = 150
            let alarm = EKAlarm()
            alarm.structuredLocation = location
            alarm.proximity = .enter
            reminder.addAlarm(alarm)
        }
        do { try store.save(reminder, commit: true) } catch { throw CalendarFailure.failed }
        return ReminderItem(id: reminder.calendarItemIdentifier, title: title, due: due, list: list.title, place: place?.rawValue)
    }

    func moveEvent(id: String, expectStart: Date, start: Date, end: Date) throws -> CalendarEvent {
        guard access(.event) else { throw CalendarFailure.calendarAccessDenied }
        // The occurrence is found by its id at the start the user was asked about.
        let window = store.predicateForEvents(withStart: expectStart.addingTimeInterval(-60), end: expectStart.addingTimeInterval(86400), calendars: nil)
        let candidates = store.events(matching: window)
        guard let event = candidates.first(where: { Self.hostId($0) == id }) else {
            let base = id.split(separator: "@").first.map(String.init)
            throw candidates.contains { $0.eventIdentifier == base } ? CalendarFailure.eventChanged : CalendarFailure.eventNotFound
        }
        guard abs(event.startDate.timeIntervalSince(expectStart)) < 1 else { throw CalendarFailure.eventChanged }
        guard event.calendar?.allowsContentModifications == true else { throw CalendarFailure.readOnlyCalendar }
        event.startDate = start
        event.endDate = end
        // One occurrence only; a recurring series is never rewritten.
        do { try store.save(event, span: .thisEvent, commit: true) } catch { throw CalendarFailure.failed }
        return convert(event)
    }

    /// The home address on the user's own contact card ("My Card"), geocoded
    /// by Apple's geocoder. Only the address leaves the Mac, to Apple.
    private func homeLocation() throws -> CLLocation {
        if let home { return home }
        let contacts = CNContactStore()
        if CNContactStore.authorizationStatus(for: .contacts) == .notDetermined {
            _ = wait { done in contacts.requestAccess(for: .contacts) { ok, _ in done(ok) } }
        }
        guard CNContactStore.authorizationStatus(for: .contacts) == .authorized,
              let me = try? contacts.unifiedMeContactWithKeys(toFetch: [CNContactPostalAddressesKey as CNKeyDescriptor]),
              let address = me.postalAddresses.first(where: { $0.label == CNLabelHome })?.value else {
            throw CalendarFailure.homeUnavailable
        }
        let placemarks: [CLPlacemark]? = wait { done in
            CLGeocoder().geocodePostalAddress(address) { found, _ in done(found ?? []) }
        }
        guard let location = placemarks?.first?.location else { throw CalendarFailure.homeUnavailable }
        home = location
        return location
    }
}
