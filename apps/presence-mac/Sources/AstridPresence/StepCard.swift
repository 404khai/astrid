import AppKit
import PresenceCore

/// ADR 0038: an operator step card. Astrid's eyes, "Step 3 of 4", the step in
/// plain words and its buttons ("Do it", "Show me", "Done", ...), placed beside
/// the target. The panel takes clicks without activating Astrid, so the app
/// the user is working in keeps keyboard focus (which "Do it" typing needs).
/// It is this process's window, so looks never capture it.
final class StepCardPanel: NSPanel {
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

private final class FirstMouseView: NSView {
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
    override var isFlipped: Bool { true }
}

private final class CardButton: NSButton {
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
}

final class StepCard {
    struct Content {
        var id: Int
        var step: Int?
        var total: Int?
        var text: String
        var buttons: [(id: String, label: String)]
        var target: CGPoint?
        var display: Int
    }

    private var panel: StepCardPanel?
    private var shown: Content?
    var onChoice: ((Int, String) -> Void)?

    private static let width: CGFloat = 340
    private static let padding: CGFloat = 20

    /// Shows a card, replacing any other. Returns false for a malformed card.
    @discardableResult
    func show(_ fields: [String: Any], layout: DisplayLayout) -> Bool {
        guard let id = (fields["id"] as? NSNumber)?.intValue, let text = fields["text"] as? String,
              !text.isEmpty, text.count <= 1_000,
              let rawButtons = fields["buttons"] as? [[String: Any]], (1...4).contains(rawButtons.count) else { return false }
        let buttons = rawButtons.compactMap { b -> (String, String)? in
            guard let id = b["id"] as? String, let label = b["label"] as? String, label.count <= 24 else { return nil }
            return (id, label)
        }
        guard buttons.count == rawButtons.count else { return false }
        let display = (fields["display"] as? NSNumber)?.intValue ?? 0
        var target: CGPoint?
        if let x = (fields["x"] as? NSNumber)?.doubleValue, let y = (fields["y"] as? NSNumber)?.doubleValue {
            target = CGPoint(x: x, y: y)
        }
        let content = Content(
            id: id, step: (fields["step"] as? NSNumber)?.intValue, total: (fields["total"] as? NSNumber)?.intValue,
            text: text, buttons: buttons, target: target, display: display)
        present(content, layout: layout)
        return true
    }

    func hide(id: Int? = nil) {
        if let id, shown?.id != id { return }
        panel?.orderOut(nil)
        panel = nil
        shown = nil
    }

    /// Temporarily out of the way (while "Do it" posts input), then back.
    func setHidden(_ hidden: Bool) {
        if hidden { panel?.orderOut(nil) } else { panel?.orderFrontRegardless() }
    }

    private func present(_ content: Content, layout: DisplayLayout) {
        hide()
        let screens = NSScreen.screens
        guard layout.displays.indices.contains(content.display), screens.indices.contains(content.display) else { return }
        let display = layout.displays[content.display], screen = screens[content.display]
        let (root, size, margin) = makeView(content)
        let local = StepCardLayout.origin(card: size, target: content.target, display: display.frame.size)
        // Display-local, y down → Cocoa screen coordinates, y up.
        let frame = NSRect(
            x: screen.frame.minX + local.x - margin,
            y: screen.frame.maxY - local.y - size.height - margin,
            width: size.width + 2 * margin, height: size.height + 2 * margin)
        root.frame = NSRect(origin: .zero, size: frame.size)

        let panel = StepCardPanel(contentRect: frame, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.level = .statusBar
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary, .ignoresCycle]
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.becomesKeyOnlyIfNeeded = true
        panel.contentView = root
        panel.setFrame(frame, display: true)
        panel.orderFrontRegardless()
        self.panel = panel
        shown = content
    }

    /// Debug: renders a sample card offscreen to a PNG (never the screen).
    static func renderSample(to url: URL) throws {
        let card = StepCard()
        let (root, _, _) = card.makeView(Content(
            id: 1, step: 3, total: 4, text: "Your site needs a bucket. “Do it”: I'll click there, then look again to check.",
            buttons: [("do", "Do it"), ("show", "Show me"), ("stop", "Stop")], target: nil, display: 0))
        root.wantsLayer = true
        root.layer?.backgroundColor = NSColor(srgbRed: 0.93, green: 0.94, blue: 0.95, alpha: 1).cgColor
        root.layoutSubtreeIfNeeded()
        guard let rep = root.bitmapImageRepForCachingDisplay(in: root.bounds) else { throw CocoaError(.fileWriteUnknown) }
        root.cacheDisplay(in: root.bounds, to: rep)
        try rep.representation(using: .png, properties: [:])?.write(to: url)
    }

    /// The card's view (with a shadow margin), the card size and that margin.
    private func makeView(_ content: Content) -> (NSView, NSSize, CGFloat) {
        let root = FirstMouseView()
        root.wantsLayer = true
        let card = NSView()
        card.wantsLayer = true
        card.layer?.backgroundColor = NSColor.white.cgColor
        card.layer?.cornerRadius = 22
        card.layer?.borderColor = NSColor(white: 0, alpha: 0.06).cgColor
        card.layer?.borderWidth = 1
        root.addSubview(card)

        let eyes = EyesView(frame: NSRect(x: 0, y: 0, width: 20, height: 16))
        let heading = NSTextField(labelWithString: content.step.map { s in content.total.map { "Step \(s) of \($0)" } ?? "Step \(s)" } ?? "Astrid")
        heading.font = .systemFont(ofSize: 15, weight: .semibold)
        heading.textColor = NSColor(white: 0.45, alpha: 1)
        let header = NSStackView(views: [eyes, heading])
        header.spacing = 10
        header.alignment = .centerY

        let body = NSTextField(wrappingLabelWithString: content.text)
        body.font = .systemFont(ofSize: 17, weight: .medium)
        body.textColor = NSColor(white: 0.07, alpha: 1)
        body.preferredMaxLayoutWidth = Self.width - 2 * Self.padding
        body.isSelectable = false

        let buttons = content.buttons.enumerated().map { index, button in
            makeButton(button.label, primary: index == 0 && button.id != "stop" && button.id != "close") { [weak self] in
                guard let self, self.shown?.id == content.id else { return }
                self.onChoice?(content.id, button.id)
                self.hide()
            }
        }
        let row = NSStackView(views: buttons)
        row.spacing = 10

        let stack = NSStackView(views: [header, body, row])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 14
        stack.edgeInsets = NSEdgeInsets(top: Self.padding, left: Self.padding, bottom: Self.padding, right: Self.padding)
        stack.translatesAutoresizingMaskIntoConstraints = false
        card.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: card.leadingAnchor),
            stack.topAnchor.constraint(equalTo: card.topAnchor),
            stack.widthAnchor.constraint(equalToConstant: Self.width),
        ])
        stack.layoutSubtreeIfNeeded()
        let size = NSSize(width: Self.width, height: ceil(stack.fittingSize.height))

        // Shadow margin around the card inside the panel.
        let margin: CGFloat = 24
        card.frame = NSRect(x: margin, y: margin, width: size.width, height: size.height)
        card.layer?.shadowColor = NSColor.black.cgColor
        card.layer?.shadowOpacity = 0.16
        card.layer?.shadowRadius = 18
        card.layer?.shadowOffset = CGSize(width: 0, height: -6)
        card.layer?.masksToBounds = false
        root.frame = NSRect(x: 0, y: 0, width: size.width + 2 * margin, height: size.height + 2 * margin)
        return (root, size, margin)
    }

    private func makeButton(_ title: String, primary: Bool, action: @escaping () -> Void) -> NSButton {
        let button = CardButton(title: title, target: nil, action: nil)
        button.isBordered = false
        button.wantsLayer = true
        button.layer?.cornerRadius = 14
        button.layer?.backgroundColor = (primary ? NSColor(white: 0.07, alpha: 1) : NSColor(srgbRed: 0.94, green: 0.93, blue: 0.91, alpha: 1)).cgColor
        button.attributedTitle = NSAttributedString(string: title, attributes: [
            .font: NSFont.systemFont(ofSize: 16, weight: .semibold),
            .foregroundColor: primary ? NSColor.white : NSColor(white: 0.1, alpha: 1),
        ])
        button.translatesAutoresizingMaskIntoConstraints = false
        let width = ceil(button.attributedTitle.size().width) + 44
        NSLayoutConstraint.activate([
            button.heightAnchor.constraint(equalToConstant: 46),
            button.widthAnchor.constraint(equalToConstant: width),
        ])
        button.setAccessibilityLabel(title)
        let target = ClosureTarget(action)
        button.target = target
        button.action = #selector(ClosureTarget.fire)
        objc_setAssociatedObject(button, &ClosureTarget.key, target, .OBJC_ASSOCIATION_RETAIN)
        return button
    }
}

private final class ClosureTarget: NSObject {
    static var key = 0
    let action: () -> Void
    init(_ action: @escaping () -> Void) { self.action = action }
    @objc func fire() { action() }
}

/// Astrid's eyes from the mark, as the card's avatar: two upright rounded
/// bars in the arch blue (the mark's own eye colour is for dark arches).
private final class EyesView: NSView {
    override var intrinsicContentSize: NSSize { NSSize(width: 20, height: 16) }
    override var isFlipped: Bool { true }
    override func draw(_ dirtyRect: NSRect) {
        NSColor(hex: AstridArt.archGradient[0]).setFill()
        for x in [1.0, 11.0] {
            NSBezierPath(roundedRect: NSRect(x: x, y: 1, width: 8, height: 14), xRadius: 3, yRadius: 3).fill()
        }
    }
}
