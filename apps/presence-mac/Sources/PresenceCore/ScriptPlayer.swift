import Foundation

/// Sequences queued events against a clock. `wait` holds later events; the
/// next deadline is measured from the previous one so ticks never add drift.
public struct ScriptPlayer {
    private var queue: [PresenceEvent] = []
    private var head = 0
    private var resumeAt: Double?

    public init() {}

    public mutating func enqueue(_ event: PresenceEvent) {
        queue.append(event)
    }

    public mutating func enqueue(contentsOf events: [PresenceEvent]) {
        queue.append(contentsOf: events)
    }

    /// True when nothing is queued or waiting.
    public var isIdle: Bool { head == queue.count && resumeAt == nil }

    /// Events due at `now`, in order. `wait` events are consumed, not returned.
    public mutating func drain(now: Double) -> [PresenceEvent] {
        var clock = now
        if let deadline = resumeAt {
            guard now >= deadline else { return [] }
            clock = deadline
            resumeAt = nil
        }
        var due: [PresenceEvent] = []
        while head < queue.count {
            let event = queue[head]
            head += 1
            if case let .wait(ms) = event {
                let deadline = clock + Double(ms) / 1000
                if deadline > now {
                    resumeAt = deadline
                    break
                }
                clock = deadline
            } else {
                due.append(event)
            }
        }
        if head == queue.count {
            queue.removeAll(keepingCapacity: true)
            head = 0
        }
        return due
    }
}

/// Splits a byte stream into lines without ever buffering more than one
/// over-long line's worth of bytes.
public struct LineAccumulator {
    public enum Item: Equatable {
        case line(String)
        case overflow
    }

    private var buffer: [UInt8] = []
    private var discarding = false
    private let maxBytes: Int

    public init(maxBytes: Int = PresenceProtocol.Limits.maxLineBytes) {
        self.maxBytes = maxBytes
    }

    public mutating func feed(_ bytes: some Sequence<UInt8>) -> [Item] {
        var items: [Item] = []
        for byte in bytes {
            if byte == UInt8(ascii: "\n") {
                if discarding {
                    discarding = false
                } else {
                    items.append(.line(String(decoding: buffer, as: UTF8.self)))
                }
                buffer.removeAll(keepingCapacity: true)
            } else if !discarding {
                buffer.append(byte)
                if buffer.count > maxBytes {
                    items.append(.overflow)
                    buffer.removeAll(keepingCapacity: true)
                    discarding = true
                }
            }
        }
        return items
    }

    /// Flushes a final unterminated line at end of input.
    public mutating func finish() -> [Item] {
        defer { buffer.removeAll(); discarding = false }
        return buffer.isEmpty || discarding ? [] : [.line(String(decoding: buffer, as: UTF8.self))]
    }
}
