import AppKit
import SwiftUI

/// Astrid's desktop app (ADR 0037): its own process and bundle, a normal Dock
/// app. It never starts, signals or waits on Astrid Presence, so closing it,
/// quitting it or removing it from the Dock leaves presence running.
struct DesktopApp: App {
    static var configuration: DesktopConfig!

    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @StateObject private var model = ChatModel(config: DesktopApp.configuration)

    var body: some Scene {
        // One window: the app is single-window for this test run.
        Window("Astrid", id: "main") {
            ContentView(model: model)
                .frame(minWidth: 760, minHeight: 520)
                .onAppear { delegate.model = model }
        }
        .defaultSize(width: 1120, height: 780)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Chat") { model.newChat() }
                    .keyboardShortcut("n")
            }
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    weak var model: ChatModel?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        NSApp.activate()
        // Debug: ASTRID_DESKTOP_ASK="text" sends one ask in the selected chat
        // once the window is up, to exercise the runtime child without typing.
        if let text = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_ASK"] {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
                self?.model?.draft = text
                self?.model?.send()
            }
        }
        // Debug: ASTRID_DESKTOP_CLOSE_AFTER=seconds closes the window as its
        // close button would, to check that closing quits only this app.
        if let delay = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_CLOSE_AFTER"].flatMap(Double.init) {
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
                NSApp.windows.first { $0.isVisible && $0.canBecomeMain }?.performClose(nil)
            }
        }
        if let path = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_SNAPSHOT"] {
            DispatchQueue.main.asyncAfter(deadline: .now() + 4) { Self.snapshot(to: URL(fileURLWithPath: path)) }
        }
    }

    /// Closing the window quits this app only; presence is another process.
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        model?.shutdown()
    }

    /// Debug: saves this app's own window content (never the screen) as a PNG
    /// and prints the window number, for `screencapture -l` of this window.
    private static func snapshot(to url: URL) {
        guard let window = NSApp.windows.first(where: { $0.isVisible && $0.canBecomeMain }),
              let view = window.contentView?.superview ?? window.contentView,
              let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { return }
        view.cacheDisplay(in: view.bounds, to: rep)
        try? rep.representation(using: .png, properties: [:])?.write(to: url)
        print("astrid-desktop window \(window.windowNumber) frame \(NSStringFromRect(window.frame)) snapshot \(url.path)")
        fflush(stdout)
    }
}
