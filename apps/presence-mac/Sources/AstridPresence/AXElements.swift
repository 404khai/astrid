import AppKit
import ApplicationServices
import PresenceCore

/// ADR 0038: the frontmost app's interactive elements, read through macOS
/// Accessibility with each operator look, so the model can name a control
/// ("Like", "Subscribe") and Astrid acts on its exact frame instead of a
/// guessed pixel. Labels only: text field values are never read.
enum AXElements {
    static let maxElements = 150
    private static let maxVisited = 4000
    private static let maxDepth = 60
    private static let roles: Set<String> = [
        "AXButton", "AXLink", "AXCheckBox", "AXRadioButton", "AXPopUpButton", "AXMenuButton",
        "AXMenuItem", "AXMenuBarItem", "AXTextField", "AXTextArea", "AXSearchField", "AXComboBox",
        "AXTab", "AXSlider", "AXDisclosureTriangle", "AXIncrementor", "AXSwitch", "AXToggle",
    ]

    /// The last app the user was in that isn't Astrid (the ask panel can be
    /// frontmost while they type).
    private(set) static var lastOtherApp: NSRunningApplication?
    private static var observer: NSObjectProtocol?

    static func startTracking() {
        let own = ProcessInfo.processInfo.processIdentifier
        if let front = NSWorkspace.shared.frontmostApplication, front.processIdentifier != own { lastOtherApp = front }
        observer = NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.didActivateApplicationNotification, object: nil, queue: .main
        ) { note in
            guard let app = note.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
                  app.processIdentifier != own else { return }
            lastOtherApp = app
        }
    }

    /// The app operator works in: the frontmost one, or the last one before Astrid.
    static func targetApp() -> NSRunningApplication? {
        let own = ProcessInfo.processInfo.processIdentifier
        if let front = NSWorkspace.shared.frontmostApplication, front.processIdentifier != own { return front }
        return lastOtherApp
    }

    /// Elements of the target app's windows on `display`, in display points:
    /// `[[id, role, label, x, y, w, h], ...]`. Nil without Accessibility access.
    /// Call off the main thread: Accessibility can be slow on large pages.
    static func collect(display: DisplayInfo, app: NSRunningApplication?) -> [[Any]]? {
        guard AXIsProcessTrusted(), let app else { return nil }
        let root = AXUIElementCreateApplication(app.processIdentifier)
        AXUIElementSetMessagingTimeout(root, 0.25)
        // Chromium and Electron build their tree for assistive apps only on request.
        AXUIElementSetAttributeValue(root, "AXManualAccessibility" as CFString, kCFBooleanTrue)
        let bounds = display.frame
        var out: [[Any]] = []
        var visited = 0
        var stack: [(AXUIElement, Int)] = (array(root, kAXWindowsAttribute) ?? []).map { ($0, 0) }
        while let (element, depth) = stack.popLast(), out.count < maxElements, visited < maxVisited {
            visited += 1
            let role = string(element, kAXRoleAttribute) ?? ""
            if roles.contains(role), let frame = frame(element), frame.width >= 4, frame.height >= 4 {
                let clipped = frame.intersection(bounds)
                if !clipped.isNull, clipped.width >= 4, clipped.height >= 4 {
                    let label = [kAXTitleAttribute, kAXDescriptionAttribute, kAXHelpAttribute, "AXPlaceholderValue"]
                        .lazy.compactMap { string(element, $0) }.first { !$0.isEmpty } ?? ""
                    out.append([out.count + 1, role, String(label.prefix(120)),
                                Int((clipped.minX - bounds.minX).rounded()), Int((clipped.minY - bounds.minY).rounded()),
                                Int(clipped.width.rounded(.down)), Int(clipped.height.rounded(.down))])
                }
            }
            if depth < maxDepth {
                // Reverse so the stack visits children in reading order.
                for child in (array(element, kAXChildrenAttribute) ?? []).reversed() { stack.append((child, depth + 1)) }
            }
        }
        return out
    }

    private static func value(_ element: AXUIElement, _ attribute: String) -> CFTypeRef? {
        var value: CFTypeRef?
        return AXUIElementCopyAttributeValue(element, attribute as CFString, &value) == .success ? value : nil
    }

    private static func string(_ element: AXUIElement, _ attribute: String) -> String? {
        (value(element, attribute) as? String)?.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func array(_ element: AXUIElement, _ attribute: String) -> [AXUIElement]? {
        value(element, attribute) as? [AXUIElement]
    }

    /// Global Quartz frame (top-left origin), as Accessibility reports it.
    private static func frame(_ element: AXUIElement) -> CGRect? {
        guard let position = value(element, kAXPositionAttribute), let size = value(element, kAXSizeAttribute),
              CFGetTypeID(position) == AXValueGetTypeID(), CFGetTypeID(size) == AXValueGetTypeID() else { return nil }
        var origin = CGPoint.zero, extent = CGSize.zero
        guard AXValueGetValue(position as! AXValue, .cgPoint, &origin),
              AXValueGetValue(size as! AXValue, .cgSize, &extent) else { return nil }
        return CGRect(origin: origin, size: extent)
    }
}
