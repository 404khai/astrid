import AppKit
import DesktopCore
import SwiftUI

/// Settings, after the maintainer's Rex app: a searchable list of pages on
/// the left with coloured icon tiles, the page on the right in grouped cards.
/// Shown as a sheet over the main window (⌘,).
struct SettingsView: View {
    @ObservedObject var model: ChatModel
    @ObservedObject var settings: AppSettings
    @Environment(\.palette) private var palette
    @Environment(\.dismiss) private var dismiss
    @State private var page: Page = .general
    @State private var search = ""

    enum Page: String, CaseIterable, Identifiable {
        case general = "General"
        case themes = "Themes"
        var id: String { rawValue }

        var symbol: String { self == .general ? "gearshape.fill" : "paintpalette.fill" }
        var tile: [Color] {
            self == .general ? [Color(white: 0.62), Color(white: 0.42)] : [.orange, .pink, .purple, .blue]
        }
        /// Words that find the page from the search field.
        var keywords: String {
            self == .general ? "general model mail gmail apple chats folder" : "themes appearance light dark colors sidebar"
        }
    }

    private var pages: [Page] {
        let words = search.lowercased().split(separator: " ")
        return Page.allCases.filter { page in
            words.allSatisfy { (page.rawValue.lowercased() + " " + page.keywords).contains($0) }
        }
    }

    var body: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 8) {
                    Image(systemName: "magnifyingglass").foregroundStyle(palette.secondary)
                    TextField("Search Settings", text: $search).textFieldStyle(.plain)
                }
                .padding(.horizontal, 12)
                .frame(height: 36)
                .glassEffect(.regular, in: .capsule)
                .padding(.bottom, 10)
                ForEach(pages) { item in
                    Button { page = item } label: {
                        HStack(spacing: 10) {
                            Image(systemName: item.symbol)
                                .font(.system(size: 13, weight: .semibold))
                                .foregroundStyle(.white)
                                .frame(width: 26, height: 26)
                                .background(LinearGradient(colors: item.tile, startPoint: .topLeading, endPoint: .bottomTrailing),
                                            in: .rect(cornerRadius: 7, style: .continuous))
                            Text(item.rawValue)
                            Spacer()
                        }
                        .padding(.horizontal, 8)
                        .frame(height: 38)
                        .background {
                            if item == page {
                                RoundedRectangle(cornerRadius: 9, style: .continuous).fill(palette.text.opacity(0.08))
                            }
                        }
                        .contentShape(.rect)
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(item == page ? .isSelected : [])
                }
                Spacer()
            }
            .padding(14)
            .frame(width: 230)
            .background(palette.sidebar.mix(with: palette.content, by: 0.35))

            VStack(spacing: 0) {
                HStack {
                    Text(page.rawValue).font(.title3.weight(.semibold))
                    Spacer()
                    Button("Done") { dismiss() }
                        .buttonStyle(.glass)
                        .keyboardShortcut(.cancelAction)
                }
                .padding(.horizontal, 22)
                .frame(height: 58)
                ScrollView {
                    Group {
                        switch page {
                        case .general: GeneralPage(model: model, settings: settings)
                        case .themes: ThemesPage(settings: settings)
                        }
                    }
                    .padding(.horizontal, 22)
                    .padding(.bottom, 22)
                }
            }
            .background(palette.content)
        }
        .foregroundStyle(palette.text)
        .frame(width: 820, height: 580)
        .onAppear {
            // Debug: ASTRID_DESKTOP_SETTINGS_PAGE=general|themes opens that page.
            if let raw = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_SETTINGS_PAGE"], let start = Page.allCases.first(where: { $0.rawValue.lowercased() == raw }) {
                page = start
            }
        }
    }
}

/// A Rex-style grouped card.
struct SettingsGroup<Content: View>: View {
    @ViewBuilder var content: Content
    @Environment(\.palette) private var palette

    var body: some View {
        VStack(alignment: .leading, spacing: 12) { content }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(palette.text.opacity(palette.spec.dark ? 0.045 : 0.035), in: .rect(cornerRadius: 14, style: .continuous))
    }
}

struct SectionTitle: View {
    var title: String
    var note: String?
    @Environment(\.palette) private var palette

    var body: some View {
        HStack(spacing: 6) {
            Text(title).font(.headline)
            if let note { Text("— \(note)").font(.headline).foregroundStyle(palette.secondary) }
        }
        .padding(.top, 14)
        .padding(.leading, 4)
    }
}

struct ThemesPage: View {
    @ObservedObject var settings: AppSettings
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SettingsGroup {
                HStack {
                    Text("Appearance")
                    Spacer()
                    Picker("Appearance", selection: $settings.appearance) {
                        Text("System").tag(Appearance.system)
                        Text("Light").tag(Appearance.light)
                        Text("Dark").tag(Appearance.dark)
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .frame(width: 240)
                }
            }
            SectionTitle(title: "Light Mode", note: scheme == .light ? "Active" : nil)
            ThemeRow(themes: Themes.light, selection: $settings.lightTheme)
            SectionTitle(title: "Dark Mode", note: scheme == .dark ? "Active" : nil)
            ThemeRow(themes: Themes.dark, selection: $settings.darkTheme)
            Text("A theme colours the sidebar and the chat. Drawings keep their dark canvas.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .padding(.top, 6)
                .padding(.leading, 4)
        }
    }
}

/// The default theme, a divider, then the built-ins, as in Rex.
struct ThemeRow: View {
    var themes: [ThemeSpec]
    @Binding var selection: String
    @Environment(\.palette) private var palette

    var body: some View {
        SettingsGroup {
            HStack(alignment: .top, spacing: 14) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Default").font(.callout).foregroundStyle(palette.secondary)
                    ThemeCard(theme: themes[0], selected: selection == themes[0].id) { selection = themes[0].id }
                }
                Rectangle().fill(palette.text.opacity(0.12)).frame(width: 1, height: 96).padding(.top, 34)
                VStack(alignment: .leading, spacing: 8) {
                    Text("Built-in").font(.callout).foregroundStyle(palette.secondary)
                    ScrollView(.horizontal) {
                        HStack(spacing: 12) {
                            ForEach(themes.dropFirst()) { theme in
                                ThemeCard(theme: theme, selected: selection == theme.id) { selection = theme.id }
                            }
                        }
                        .padding(3)
                    }
                    .scrollIndicators(.never)
                    .padding(-3)
                }
            }
        }
    }
}

/// A theme preview: "Aa" in the theme's text on its content colour, a
/// sidebar chip and the accent, then its swatches as overlapping dots.
struct ThemeCard: View {
    var theme: ThemeSpec
    var selected: Bool
    var choose: () -> Void
    @Environment(\.palette) private var palette

    var body: some View {
        let p = Palette(spec: theme)
        Button(action: choose) {
            VStack(spacing: 8) {
                VStack(alignment: .leading, spacing: 10) {
                    HStack(spacing: 8) {
                        Text("Aa").font(.system(size: 24, weight: .medium)).foregroundStyle(p.text)
                        Spacer()
                        VStack(alignment: .leading, spacing: 3) {
                            ForEach(0..<3) { _ in Capsule().fill(p.sidebarText.opacity(0.7)).frame(width: 16, height: 2.5) }
                        }
                        .frame(width: 30, height: 26)
                        .background(p.sidebar, in: .rect(cornerRadius: 6))
                        Capsule().fill(p.accent).frame(width: 10, height: 26)
                    }
                    HStack(spacing: -6) {
                        ForEach(Array(theme.swatches.enumerated()), id: \.offset) { _, hex in
                            Circle().fill(Color(hex: hex))
                                .overlay(Circle().strokeBorder(p.content, lineWidth: 1.5))
                                .frame(width: 20, height: 20)
                        }
                    }
                }
                .padding(12)
                .frame(width: 170, height: 96)
                .background(p.content, in: .rect(cornerRadius: 14, style: .continuous))
                .overlay {
                    RoundedRectangle(cornerRadius: 14, style: .continuous)
                        .strokeBorder(p.text.opacity(0.12), lineWidth: 1)
                }
                .padding(3)
                .overlay {
                    if selected {
                        RoundedRectangle(cornerRadius: 17, style: .continuous)
                            .strokeBorder(palette.accent, lineWidth: 3)
                    }
                }
                Text(theme.name)
                    .font(.callout.weight(selected ? .semibold : .regular))
                    .foregroundStyle(selected ? palette.text : palette.secondary)
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(theme.name) theme")
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

struct GeneralPage: View {
    @ObservedObject var model: ChatModel
    @ObservedObject var settings: AppSettings
    @Environment(\.palette) private var palette

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle(title: "Asks")
            SettingsGroup {
                HStack {
                    Text("Model")
                    Spacer()
                    TextField("claude-code/sonnet", text: $settings.storedModel)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 280)
                        .disabled(settings.modelOverride != nil)
                }
                if let forced = settings.modelOverride {
                    Text("Set to \(forced) by --model at launch.").font(.caption).foregroundStyle(palette.secondary)
                }
                Divider()
                HStack {
                    Text("Mail")
                    Spacer()
                    Picker("Mail", selection: $settings.storedMail) {
                        Text("Gmail").tag("gmail")
                        Text("Apple Mail").tag("apple")
                        Text("Both").tag("all")
                        Text("None").tag("none")
                    }
                    .labelsHidden()
                    .frame(width: 180)
                    .disabled(settings.mailOverride != nil)
                }
                Text("Mail is read-only, and Astrid asks before every search or read. Changes apply from your next ask.")
                    .font(.caption)
                    .foregroundStyle(palette.secondary)
            }
            SectionTitle(title: "Chats")
            SettingsGroup {
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Saved on this Mac")
                        Text(model.config.chats.path.replacingOccurrences(of: NSHomeDirectory(), with: "~"))
                            .font(.caption)
                            .foregroundStyle(palette.secondary)
                            .textSelection(.enabled)
                    }
                    Spacer()
                    Button("Show in Finder") {
                        try? FileManager.default.createDirectory(at: model.config.chats, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
                        NSWorkspace.shared.activateFileViewerSelecting([model.config.chats])
                    }
                    .buttonStyle(.glass)
                }
            }
        }
    }
}
