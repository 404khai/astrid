import DesktopCore
import SwiftUI

/// The app's preferences, kept in its own UserDefaults domain: appearance,
/// the light and dark themes, the sidebar, and the model and mail source for
/// asks. A launch flag (`--model`, `--mail`) wins over the stored value.
final class AppSettings: ObservableObject {
    private let defaults = UserDefaults.standard
    let modelOverride: String?
    let mailOverride: String?

    @Published var appearance: Appearance { didSet { defaults.set(appearance.rawValue, forKey: "appearance") } }
    @Published var lightTheme: String { didSet { defaults.set(lightTheme, forKey: "theme.light") } }
    @Published var darkTheme: String { didSet { defaults.set(darkTheme, forKey: "theme.dark") } }
    @Published var sidebarShown: Bool { didSet { defaults.set(sidebarShown, forKey: "sidebar.shown") } }
    /// The docked sidebar's width, dragged from its right edge.
    @Published var sidebarWidth: Double { didSet { defaults.set(sidebarWidth, forKey: "sidebar.width") } }
    @Published var storedModel: String { didSet { defaults.set(storedModel, forKey: "ask.model") } }
    @Published var storedMail: String { didSet { defaults.set(storedMail, forKey: "ask.mail") } }
    /// The row of blobs above the sidebar's foot, and the one picked.
    @Published var showBlobs: Bool { didSet { defaults.set(showBlobs, forKey: "blobs.show") } }
    @Published var blob: String? { didSet { defaults.set(blob, forKey: "blobs.selected") } }
    /// Which bots show in the row, in order (file names).
    @Published var blobsShown: [String] { didSet { defaults.set(blobsShown, forKey: "blobs.shown") } }
    /// Not stored: whether the settings sheet is open.
    @Published var showSettings = false

    static let mailSources = ["gmail", "apple", "all", "none"]
    static let sidebarMin = 200.0
    static let sidebarMax = 380.0
    static let sidebarDefault = 264.0
    /// The window buttons show only on a docked sidebar at least this wide.
    static let trafficLightsMinWidth = 236.0

    var trafficLightsVisible: Bool { sidebarShown && sidebarWidth >= Self.trafficLightsMinWidth }

    init(modelOverride: String?, mailOverride: String?) {
        self.modelOverride = modelOverride
        self.mailOverride = mailOverride
        appearance = Appearance(rawValue: defaults.string(forKey: "appearance") ?? "") ?? .system
        lightTheme = defaults.string(forKey: "theme.light") ?? Themes.defaultLight
        darkTheme = defaults.string(forKey: "theme.dark") ?? Themes.defaultDark
        sidebarShown = defaults.object(forKey: "sidebar.shown") as? Bool ?? true
        let width = defaults.object(forKey: "sidebar.width") as? Double ?? Self.sidebarDefault
        sidebarWidth = min(max(width, Self.sidebarMin), Self.sidebarMax)
        showBlobs = defaults.object(forKey: "blobs.show") as? Bool ?? true
        blob = defaults.string(forKey: "blobs.selected")
        blobsShown = defaults.stringArray(forKey: "blobs.shown") ?? Blobs.defaults
        storedModel = defaults.string(forKey: "ask.model") ?? "claude-code/sonnet"
        let mail = defaults.string(forKey: "ask.mail") ?? "gmail"
        storedMail = Self.mailSources.contains(mail) ? mail : "gmail"
    }

    var model: String { modelOverride ?? storedModel }
    var mail: String { mailOverride ?? storedMail }

    func theme(dark: Bool) -> ThemeSpec {
        Themes.resolve(dark ? darkTheme : lightTheme, dark: dark)
    }

    var colorScheme: ColorScheme? {
        switch appearance {
        case .system: nil
        case .light: .light
        case .dark: .dark
        }
    }
}

extension Color {
    init(hex: String) {
        let c = hexComponents(hex) ?? (0.5, 0.5, 0.5)
        self.init(.sRGB, red: c.r, green: c.g, blue: c.b)
    }
}

/// The active theme's colours, handed down through the environment.
struct Palette {
    var spec: ThemeSpec
    var sidebar: Color { Color(hex: spec.sidebar) }
    var sidebarText: Color { Color(hex: spec.sidebarText) }
    var content: Color { Color(hex: spec.content) }
    var text: Color { Color(hex: spec.text) }
    var secondary: Color { Color(hex: spec.secondary) }
    var accent: Color { Color(hex: spec.accent) }
    var bubble: Color { Color(hex: spec.bubble) }
}

private struct PaletteKey: EnvironmentKey {
    static let defaultValue = Palette(spec: Themes.resolve(nil, dark: true))
}

extension EnvironmentValues {
    var palette: Palette {
        get { self[PaletteKey.self] }
        set { self[PaletteKey.self] = newValue }
    }
}
