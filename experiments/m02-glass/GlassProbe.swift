// M02-00 only: no runtime, IPC, credentials, file picker or external navigation.
import AppKit
import WebKit

@MainActor final class Backdrop: NSView {
    override func draw(_ dirtyRect: NSRect) {
        for i in 0..<12 {
            NSColor(calibratedHue: CGFloat(i) / 12, saturation: 0.7, brightness: 0.7, alpha: 1).setFill()
            NSRect(x: CGFloat(i) * bounds.width / 12, y: 0, width: bounds.width / 12 + 1, height: bounds.height).fill()
        }
    }
}
@MainActor final class Probe: NSObject, NSApplicationDelegate, WKNavigationDelegate {
    var window: NSWindow!
    var web: WKWebView!
    var surfaces: [NSView] = []
    var forceOpaque = CommandLine.arguments.contains("--opaque-fallback")
    func applicationDidFinishLaunching(_ notification: Notification) {
        let root = Backdrop(frame: NSRect(x: 0, y: 0, width: 1080, height: 660))
        window = NSWindow(contentRect: root.bounds, styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        window.title = "Astrid M02 — native material feasibility (no runtime)"
        window.contentView = root
        window.appearance = NSAppearance(named: .darkAqua)
        window.center()
        let reduced = NSWorkspace.shared.accessibilityDisplayShouldReduceTransparency
        print("os=\(ProcessInfo.processInfo.operatingSystemVersionString) reduceTransparency=\(reduced) increaseContrast=\(NSWorkspace.shared.accessibilityDisplayShouldIncreaseContrast) reduceMotion=\(NSWorkspace.shared.accessibilityDisplayShouldReduceMotion) forcedOpaque=\(forceOpaque)")
        func add(_ content: NSView, _ frame: NSRect, glass: Bool = true) {
            let surface: NSView
            if #available(macOS 26.0, *), glass, !reduced, !forceOpaque {
                let effect = NSGlassEffectView(frame: frame)
                effect.style = .regular
                effect.cornerRadius = 24
                effect.contentView = content
                surface = effect
            } else {
                let backing = NSView(frame: frame)
                backing.wantsLayer = true
                backing.layer?.backgroundColor = NSColor(calibratedWhite: 0.08, alpha: 1).cgColor
                backing.layer?.cornerRadius = 24
                content.frame = backing.bounds
                content.autoresizingMask = [.width, .height]
                backing.addSubview(content)
                surface = backing
            }
            root.addSubview(surface)
            surfaces.append(surface)
            print("surface=\(type(of: surface)) content=\(type(of: content)) frame=\(frame)")
        }
        let native = NSView(frame: NSRect(x: 0, y: 0, width: 470, height: 140))
        let label = NSTextField(labelWithString: "Native contentView · Liquid Glass\nMeister — controls, not conversation backing")
        label.frame = NSRect(x: 24, y: 50, width: 430, height: 65)
        label.font = .systemFont(ofSize: 19)
        label.textColor = .labelColor
        native.addSubview(label)
        let button = NSButton(title: "Keyboard focus test", target: nil, action: nil)
        button.frame = NSRect(x: 24, y: 12, width: 220, height: 32)
        native.addSubview(button)
        add(native, NSRect(x: 30, y: 480, width: 1020, height: 140))
        web = WKWebView(frame: NSRect(x: 0, y: 0, width: 1020, height: 190))
        web.underPageBackgroundColor = .clear // Public API. No private drawsBackground KVC.
        web.navigationDelegate = self
        web.loadHTMLString("""
        <!doctype html><meta name="viewport" content="width=device-width"><style>
        :root{color-scheme:dark}html,body{background:transparent;color:#fff;font:19px system-ui;margin:0}section{padding:24px}button,input{font:inherit;padding:8px;background:#1d202b;color:white;border:1px solid #777;border-radius:12px}p{margin:12px 0}
        </style><section><strong>WKWebView inside native contentView</strong><p>Transparent CSS + public underPageBackgroundColor. No backdrop blur.</p><input aria-label="Prototype task" placeholder="Type a task (never submitted)"><button>Focus test</button></section>
        """, baseURL: nil)
        add(web, NSRect(x: 30, y: 230, width: 1020, height: 190))
        let vibrancy = NSVisualEffectView(frame: NSRect(x: 30, y: 30, width: 1020, height: 150))
        vibrancy.material = .hudWindow
        vibrancy.blendingMode = .withinWindow
        vibrancy.state = .active
        let comparison = NSTextField(labelWithString: "NSVisualEffectView · ordinary vibrancy comparison\nThis is NOT Liquid Glass")
        comparison.frame = NSRect(x: 24, y: 35, width: 920, height: 80)
        comparison.font = .systemFont(ofSize: 19)
        vibrancy.addSubview(comparison)
        root.addSubview(vibrancy)
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        // Bounded experiment: terminate automatically; never leave an execution host.
        DispatchQueue.main.asyncAfter(deadline: .now() + 180) { NSApp.terminate(nil) }
    }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        print("webLoaded=true underPageBackgroundColor=\(String(describing: webView.underPageBackgroundColor)) nativeGlassClass=\(NSClassFromString("NSGlassEffectView") != nil)")
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}
@main struct Main {
    @MainActor static func main() {
        let app = NSApplication.shared
        let probe = Probe()
        app.setActivationPolicy(.regular)
        app.delegate = probe
        app.run()
    }
}
