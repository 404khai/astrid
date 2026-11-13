import Foundation

/// A desktop theme (ADR 0037): the colours of the sidebar (a tint over the
/// window's translucent material) and of the main content card. Colours are
/// `#rrggbb` hex so themes stay plain data and testable without SwiftUI.
public struct ThemeSpec: Equatable, Identifiable, Sendable {
    public var id: String
    public var name: String
    public var dark: Bool
    /// Tint laid over the window material behind the sidebar.
    public var sidebar: String
    /// How much of the tint covers the material, 0–1 (lower lets the desktop show).
    public var sidebarOpacity: Double
    public var sidebarText: String
    public var content: String
    public var text: String
    public var secondary: String
    public var accent: String
    /// The user's message bubble.
    public var bubble: String

    /// Swatches for the theme card, in the order they are drawn.
    public var swatches: [String] { [sidebar, content, text, secondary, accent, bubble] }
}

public enum Appearance: String, CaseIterable, Sendable {
    case system, light, dark
}

public enum Themes {
    public static let defaultLight = "astrid-day"
    public static let defaultDark = "astrid-night"

    public static let light: [ThemeSpec] = [
        ThemeSpec(id: "astrid-day", name: "Astrid Day", dark: false, sidebar: "#E6E6EC", sidebarOpacity: 0.72, sidebarText: "#1C1C1F",
                  content: "#FAFAFB", text: "#1C1C1F", secondary: "#6B6B73", accent: "#5756F5", bubble: "#E4E4FB"),
        ThemeSpec(id: "paper", name: "Paper", dark: false, sidebar: "#E9E1D3", sidebarOpacity: 0.78, sidebarText: "#2A2520",
                  content: "#FBF8F2", text: "#2A2520", secondary: "#7A6E62", accent: "#C2643B", bubble: "#F1E2D3"),
        ThemeSpec(id: "sage", name: "Sage", dark: false, sidebar: "#DCE5DC", sidebarOpacity: 0.76, sidebarText: "#1E2A21",
                  content: "#F6F9F5", text: "#1E2A21", secondary: "#62705F", accent: "#3E8A5A", bubble: "#DDEDE0"),
        ThemeSpec(id: "linen", name: "Coastal Linen", dark: false, sidebar: "#DAE4EB", sidebarOpacity: 0.76, sidebarText: "#1B2731",
                  content: "#F7FAFC", text: "#1B2731", secondary: "#5E6E7B", accent: "#2F7FA8", bubble: "#D9E9F3"),
    ]

    public static let dark: [ThemeSpec] = [
        ThemeSpec(id: "astrid-night", name: "Astrid Night", dark: true, sidebar: "#1B1B20", sidebarOpacity: 0.62, sidebarText: "#EDEDF0",
                  content: "#141416", text: "#EDEDF0", secondary: "#9A9AA2", accent: "#6575FF", bubble: "#262B4D"),
        ThemeSpec(id: "ember", name: "Ember", dark: true, sidebar: "#2B1D16", sidebarOpacity: 0.58, sidebarText: "#F3E9E2",
                  content: "#1A1412", text: "#F3E9E2", secondary: "#B09E92", accent: "#E8875B", bubble: "#3A2519"),
        ThemeSpec(id: "moss", name: "Moss", dark: true, sidebar: "#17211B", sidebarOpacity: 0.62, sidebarText: "#E6EFE8",
                  content: "#111813", text: "#E6EFE8", secondary: "#93A598", accent: "#4FA36E", bubble: "#1F3527"),
        ThemeSpec(id: "abyss", name: "Abyss", dark: true, sidebar: "#0F1724", sidebarOpacity: 0.64, sidebarText: "#E3ECF7",
                  content: "#0B111B", text: "#E3ECF7", secondary: "#8C9AAD", accent: "#39C2D7", bubble: "#123142"),
    ]

    public static var all: [ThemeSpec] { light + dark }

    /// The theme with `id` for that mode, or the mode's default when it is
    /// unknown or belongs to the other mode.
    public static func resolve(_ id: String?, dark isDark: Bool) -> ThemeSpec {
        let list = isDark ? dark : light
        return list.first { $0.id == id } ?? list[0]
    }
}

/// Parses `#rrggbb` into 0–1 components.
public func hexComponents(_ hex: String) -> (r: Double, g: Double, b: Double)? {
    guard hex.count == 7, hex.first == "#", let value = UInt32(hex.dropFirst(), radix: 16) else { return nil }
    return (Double((value >> 16) & 0xFF) / 255, Double((value >> 8) & 0xFF) / 255, Double(value & 0xFF) / 255)
}

/// Sidebar search: every word must appear in the chat's title or messages.
public enum ChatSearch {
    public static func filter(_ chats: [ChatSession], query: String) -> [ChatSession] {
        let words = query.lowercased().split(whereSeparator: \.isWhitespace)
        guard !words.isEmpty else { return chats }
        return chats.filter { chat in
            let haystack = ([chat.title] + chat.messages.map(\.text)).joined(separator: "\n").lowercased()
            return words.allSatisfy { haystack.contains($0) }
        }
    }
}
