import AppKit
import ApplicationServices
import PresenceCore

/// ADR 0038 "Do it": posts exactly one validated action as system input at a
/// global point. Needs Accessibility access for the Astrid Presence bundle;
/// without it nothing is posted and the runtime is told why.
enum Injector {
    /// Whether input may be posted, prompting for Accessibility once if not.
    static func trusted() -> Bool {
        let prompt = kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String
        return AXIsProcessTrustedWithOptions([prompt: true] as CFDictionary)
    }

    /// `point` is global Quartz coordinates (origin top-left of the primary display).
    static func perform(_ action: OperatorAction, at point: CGPoint) {
        let source = CGEventSource(stateID: .hidSystemState)
        func click() {
            CGEvent(mouseEventSource: source, mouseType: .mouseMoved, mouseCursorPosition: point, mouseButton: .left)?.post(tap: .cghidEventTap)
            CGEvent(mouseEventSource: source, mouseType: .leftMouseDown, mouseCursorPosition: point, mouseButton: .left)?.post(tap: .cghidEventTap)
            CGEvent(mouseEventSource: source, mouseType: .leftMouseUp, mouseCursorPosition: point, mouseButton: .left)?.post(tap: .cghidEventTap)
        }
        switch action {
        case .click:
            click()
        case let .type(text):
            click()
            // Let the click land before typing into the field it focused.
            usleep(120_000)
            for chunk in Array(text.utf16).chunked(16) {
                for down in [true, false] {
                    let event = CGEvent(keyboardEventSource: source, virtualKey: 0, keyDown: down)
                    chunk.withUnsafeBufferPointer { event?.keyboardSetUnicodeString(stringLength: chunk.count, unicodeString: $0.baseAddress) }
                    event?.post(tap: .cghidEventTap)
                }
            }
        case let .key(name):
            guard let code = OperatorAction.keyCodes[name] else { return }
            for down in [true, false] {
                let event = CGEvent(keyboardEventSource: source, virtualKey: CGKeyCode(code), keyDown: down)
                event?.flags = []
                event?.post(tap: .cghidEventTap)
            }
        case let .scroll(down, notches):
            CGEvent(mouseEventSource: source, mouseType: .mouseMoved, mouseCursorPosition: point, mouseButton: .left)?.post(tap: .cghidEventTap)
            CGEvent(scrollWheelEvent2Source: source, units: .line, wheelCount: 1, wheel1: Int32(down ? -notches : notches), wheel2: 0, wheel3: 0)?.post(tap: .cghidEventTap)
        }
    }
}

private extension Array {
    func chunked(_ size: Int) -> [[Element]] {
        stride(from: 0, to: count, by: size).map { Array(self[$0..<Swift.min($0 + size, count)]) }
    }
}
