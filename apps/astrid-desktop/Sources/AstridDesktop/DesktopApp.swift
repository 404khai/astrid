import AppKit
import SwiftUI

/// Astrid's desktop app (ADR 0037): its own process and bundle, a normal Dock
/// app. It never starts, signals or waits on Astrid Presence, so closing it,
/// quitting it or removing it from the Dock leaves presence running.
struct DesktopApp: App {
    static var configuration: DesktopConfig!

    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @StateObject private var settings: AppSettings
    @StateObject private var model: ChatModel

    init() {
        let config = DesktopApp.configuration!
        let settings = AppSettings(modelOverride: config.modelOverride, mailOverride: config.mailOverride)
        _settings = StateObject(wrappedValue: settings)
        _model = StateObject(wrappedValue: ChatModel(config: config, model: settings.model, mail: settings.mail))
    }

    var body: some Scene {
        // One window: the app is single-window for this test run.
        Window("Astrid", id: "main") {
            RootView(model: model, settings: settings)
                .frame(minWidth: 820, minHeight: 540)
                // The window's own material shows through the sidebar's tint.
                .containerBackground(.thinMaterial, for: .window)
                .preferredColorScheme(settings.colorScheme)
                .onAppear {
                    delegate.model = model
                    delegate.settings = settings
                }
        }
        .windowStyle(.hiddenTitleBar)
        .windowBackgroundDragBehavior(.enabled)
        .defaultSize(width: 1120, height: 780)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Chat") { model.newChat() }
                    .keyboardShortcut("n")
            }
            CommandGroup(replacing: .appSettings) {
                Button("Settings…") { settings.showSettings = true }
                    .keyboardShortcut(",")
            }
            CommandGroup(replacing: .sidebar) {
                Button(settings.sidebarShown ? "Hide Sidebar" : "Show Sidebar") { settings.sidebarShown.toggle() }
                    .keyboardShortcut("s", modifiers: [.control, .command])
            }
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    weak var model: ChatModel?
    weak var settings: AppSettings?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        NSApp.activate()
        let environment = ProcessInfo.processInfo.environment
        // Debug: ASTRID_DESKTOP_ASK="text" sends one ask in the selected chat
        // once the window is up, to exercise the runtime child without typing.
        if let text = environment["ASTRID_DESKTOP_ASK"] {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
                self?.model?.draft = text
                self?.model?.send()
            }
        }
        // Debug: ASTRID_DESKTOP_SETTINGS=1 opens Settings once the window is up.
        if environment["ASTRID_DESKTOP_SETTINGS"] != nil {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1) { [weak self] in self?.settings?.showSettings = true }
        }
        // Debug: ASTRID_DESKTOP_CLOSE_AFTER=seconds closes the window as its
        // close button would, to check that closing quits only this app.
        if let delay = environment["ASTRID_DESKTOP_CLOSE_AFTER"].flatMap(Double.init) {
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
                NSApp.windows.first { $0.isVisible && $0.canBecomeMain }?.performClose(nil)
            }
        }
        if let path = environment["ASTRID_DESKTOP_SNAPSHOT"] {
            DispatchQueue.main.asyncAfter(deadline: .now() + 4) { Self.snapshot(to: URL(fileURLWithPath: path)) }
        }
    }

    /// Closing the window quits this app only; presence is another process.
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func applicationWillTerminate(_ notification: Notification) {
        model?.shutdown()
    }

    /// Debug: saves this app's own window content (never the screen) as a PNG
    /// and prints the window number, for `screencapture -l` of this window,
    /// which also shows Liquid Glass (an in-process cache draw does not).
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
