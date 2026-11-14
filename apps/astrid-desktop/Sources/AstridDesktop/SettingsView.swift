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
        case phone = "Phone"
        case blobs = "Blobs"
        case about = "About"
        var id: String { rawValue }

        var symbol: String {
            switch self {
            case .general: "gearshape.fill"
            case .themes: "paintpalette.fill"
            case .phone: "iphone.gen3"
            case .blobs: "face.smiling"
            case .about: "info.circle.fill"
            }
        }
        var tile: [Color] {
            switch self {
            case .general: [Color(white: 0.62), Color(white: 0.42)]
            case .themes: [.orange, .pink, .purple, .blue]
            case .phone: [Color(red: 0.4, green: 0.46, blue: 1), Color(red: 0.1, green: 0.2, blue: 0.93)]
            case .blobs: [Color(white: 0.2), Color(white: 0.12)]
            case .about: [Color(red: 0, green: 0.85, blue: 0.75), Color(red: 0.1, green: 0.5, blue: 0.9)]
            }
        }
        /// Words that find the page from the search field.
        var keywords: String {
            switch self {
            case .general: "general model mail gmail apple chats folder"
            case .themes: "themes appearance light dark colors sidebar"
            case .phone: "phone android pair qr code link sync devices revoke"
            case .blobs: "blobs bots avatars characters sidebar row"
            case .about: "about version privacy credits"
            }
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
                            Group {
                                if item == .blobs, let giovanni = Blobs.icon {
                                    Image(nsImage: giovanni).resizable().frame(width: 20, height: 20)
                                } else {
                                    Image(systemName: item.symbol)
                                        .font(.system(size: 13, weight: .semibold))
                                        .foregroundStyle(.white)
                                }
                            }
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
                        case .phone: PhonePage()
                        case .blobs: BlobsPage(settings: settings)
                        case .about: AboutPage()
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
            // Debug: ASTRID_DESKTOP_SETTINGS_PAGE=general|themes|phone opens that page.
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
    @State private var custom = ""

    private var current: String { settings.modelOverride ?? settings.storedModel }
    private var locked: Bool { settings.modelOverride != nil }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle(title: "Models")
            SettingsGroup {
                ForEach(Provider.all) { provider in
                    ProviderRow(provider: provider, current: current, locked: locked) { settings.storedModel = $0 }
                    Divider()
                }
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Other model")
                        Text(Provider.of(current) == nil ? "In use: \(current)" : "Any exact model ID the runtime supports.")
                            .font(.caption)
                            .foregroundStyle(palette.secondary)
                    }
                    Spacer()
                    TextField("model id", text: $custom)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 200)
                        .onSubmit { if !custom.isEmpty { settings.storedModel = custom } }
                    Button("Use") { if !custom.isEmpty { settings.storedModel = custom } }
                        .buttonStyle(.glass)
                        .disabled(custom.isEmpty)
                }
                .disabled(locked)
                if let forced = settings.modelOverride {
                    Text("Set to \(forced) by --model at launch.").font(.caption).foregroundStyle(palette.secondary)
                }
            }
            SectionTitle(title: "Mail")
            SettingsGroup {
                HStack {
                    Text("Mail source")
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

/// One provider: its mark, name and status, how many models it offers, and a
/// menu of them; "Use" makes its chosen model Astrid's.
struct ProviderRow: View {
    var provider: Provider
    var current: String
    var locked: Bool
    var use: (String) -> Void
    @Environment(\.palette) private var palette
    @State private var choice = ""

    private var active: Bool { provider.models.contains(current) }

    var body: some View {
        HStack(alignment: .center, spacing: 12) {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 8) {
                    Group {
                        if let image = provider.image {
                            Image(nsImage: image).resizable().aspectRatio(contentMode: .fit)
                        } else {
                            Image(systemName: "cpu")
                        }
                    }
                    .frame(width: 18, height: 18)
                    Text(provider.name).font(.headline)
                    if active { Chip(text: "IN USE", tint: palette.accent) }
                    else if let status = provider.status { Chip(text: status, tint: nil) }
                }
                Text("\(provider.models.count) models available. \(provider.note)")
                    .font(.caption)
                    .foregroundStyle(palette.secondary)
            }
            Spacer()
            Picker(provider.name, selection: $choice) {
                ForEach(provider.models, id: \.self) { Text($0).tag($0) }
            }
            .labelsHidden()
            .frame(width: 190)
            .onChange(of: choice) { _, next in if active, next != current { use(next) } }
            Button(active ? "In use" : "Use") { use(choice) }
                .buttonStyle(.glass)
                .disabled(active || locked)
        }
        .disabled(locked)
        .onAppear { choice = active ? current : provider.models[0] }
        .onChange(of: current) { _, next in if provider.models.contains(next) { choice = next } }
    }
}

struct Chip: View {
    var text: String
    var tint: Color?
    @Environment(\.palette) private var palette

    var body: some View {
        Text(text)
            .font(.system(size: 10, weight: .semibold))
            .tracking(0.4)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .foregroundStyle(tint ?? palette.secondary)
            .background((tint ?? palette.text).opacity(0.12), in: .capsule)
    }
}

/// What Astrid is, its version, and where its data lives.
struct AboutPage: View {
    @Environment(\.palette) private var palette

    private var version: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "development build"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            VStack(spacing: 10) {
                TrackingLogoMark(size: 72)
                Text("Astrid").font(.title.weight(.semibold))
                Text("Version \(version)").foregroundStyle(palette.secondary)
                Text("A personal agent that sees what you show it, sketches, points and walks you through things.")
                    .multilineTextAlignment(.center)
                    .foregroundStyle(palette.secondary)
                    .frame(maxWidth: 380)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 18)
            SectionTitle(title: "Privacy")
            SettingsGroup {
                Text("Chats are saved only on this Mac. Voice is recognised and spoken on this Mac. Asks go to the model provider you choose in General. Paired phones reach this Mac only over your local network.")
                    .foregroundStyle(palette.secondary)
            }
            SectionTitle(title: "Credits")
            SettingsGroup {
                Text("The intro wave is from Distortion Waves by Jakub Antalik (MIT).")
                    .foregroundStyle(palette.secondary)
            }
        }
    }
}

/// Pairing a phone (ADR 0039): the link's switch, a two-minute QR code, and
/// the phones paired with this Mac, each one revocable.
struct PhonePage: View {
    @EnvironmentObject private var link: PhoneLink
    @Environment(\.palette) private var palette

    private var pairing: Binding<Bool> {
        Binding(get: { link.offer != nil }, set: { if !$0 { link.cancelOffer() } })
    }

    var body: some View {
        content.sheet(isPresented: pairing) { PairingSheet().environmentObject(link) }
    }

    private var content: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle(title: "Phone link")
            SettingsGroup {
                Toggle(isOn: $link.enabled) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Let paired phones connect")
                        Text("Only on this Mac's local network, and only while this app is open. Phones see your chats and ask Astrid here; their asks can't use mail.")
                            .font(.caption)
                            .foregroundStyle(palette.secondary)
                    }
                }
                .toggleStyle(.switch)
                if let problem = link.problem {
                    Text(problem).font(.caption).foregroundStyle(.red)
                }
            }
            SectionTitle(title: "Pair a phone")
            SettingsGroup {
                HStack {
                    Text("Show a QR code that pairs one phone.")
                    Spacer()
                    Button("Pair Phone") { link.newOffer() }.buttonStyle(.glass)
                }
            }
            SectionTitle(title: "Paired phones")
            SettingsGroup {
                if link.devices.isEmpty {
                    Text("None yet.").foregroundStyle(palette.secondary)
                }
                ForEach(link.devices) { device in
                    HStack {
                        Image(systemName: "iphone.gen3")
                        VStack(alignment: .leading, spacing: 2) {
                            Text(device.name)
                            Text(link.connected.contains(device.id) ? "Connected" : device.lastSeen.map { "Last seen \($0.formatted(.relative(presentation: .named)))" } ?? "Paired \(device.paired.formatted(date: .abbreviated, time: .omitted))")
                                .font(.caption)
                                .foregroundStyle(link.connected.contains(device.id) ? Color.green : palette.secondary)
                        }
                        Spacer()
                        Button("Revoke", role: .destructive) { link.revoke(device.id) }.buttonStyle(.glass)
                    }
                    if device.id != link.devices.last?.id { Divider() }
                }
            }
        }
    }
}

/// The pairing code in its own modal: Astrid's mark, the QR code with the mark
/// in its middle, how long it has left, and copy and new-code buttons.
struct PairingSheet: View {
    @EnvironmentObject private var link: PhoneLink
    @Environment(\.dismiss) private var dismiss
    @State private var copied = false

    var body: some View {
        VStack(spacing: 0) {
            TrackingLogoMark(size: 56)
                .padding(.vertical, 14)
                .background(alignment: .bottom) {
                    Ellipse().fill(.white.opacity(0.08)).frame(width: 160, height: 10).blur(radius: 6)
                }
                .padding(.top, 28)
            Text("Connect a phone to this Mac")
                .font(.system(size: 26, weight: .semibold))
                .padding(.top, 14)
            Text("Open Astrid on your phone, tap Scan QR code, and point it here.")
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.top, 8)
            Group {
                if let url = link.pairingURL, let image = PhoneLink.qrImage(url) {
                    Image(nsImage: image)
                        .interpolation(.none)
                        .resizable()
                        .frame(width: 280, height: 280)
                        .overlay {
                            AstridLogoMark(size: 46)
                                .padding(9)
                                .background(.white, in: .rect(cornerRadius: 10, style: .continuous))
                        }
                        .accessibilityLabel("Pairing QR code")
                } else {
                    Text(link.hosts.isEmpty ? "No local network found." : "Making a code…")
                        .foregroundStyle(.black.opacity(0.6))
                        .frame(width: 280, height: 280)
                }
            }
            .padding(22)
            .background(.white, in: .rect(cornerRadius: 22, style: .continuous))
            .padding(.top, 22)
            HStack {
                if let offer = link.offer {
                    TimelineView(.periodic(from: .now, by: 1)) { context in
                        let left = max(Int(offer.expires.timeIntervalSince(context.date)), 0)
                        Label("Expires in \(left / 60):\(String(format: "%02d", left % 60))", systemImage: "clock")
                    }
                }
                Spacer()
                Button {
                    guard let url = link.pairingURL else { return }
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(url, forType: .string)
                    copied = true
                } label: { Image(systemName: copied ? "checkmark" : "doc.on.doc") }
                .buttonStyle(.plain)
                .help("Copy the pairing link")
                Button { copied = false; link.newOffer() } label: { Image(systemName: "arrow.clockwise") }
                    .buttonStyle(.plain)
                    .help("New code")
                    .padding(.leading, 10)
            }
            .foregroundStyle(.secondary)
            .frame(width: 324)
            .padding(.top, 16)
            Text("Can't scan? Copy the link and open it on your phone. Each code works once.")
                .font(.callout)
                .foregroundStyle(.tertiary)
                .multilineTextAlignment(.center)
                .padding(.top, 26)
                .padding(.bottom, 24)
        }
        .padding(.horizontal, 40)
        .frame(width: 480)
        .overlay(alignment: .topTrailing) {
            Button { dismiss() } label: { Image(systemName: "xmark") }
                .buttonStyle(.glass)
                .buttonBorderShape(.circle)
                .keyboardShortcut(.cancelAction)
                .padding(14)
        }
    }
}

/// Blobs: the little bots in a row above the sidebar's foot.
struct BlobsPage: View {
    @ObservedObject var settings: AppSettings
    @Environment(\.palette) private var palette
    @State private var every: [Blob] = Blobs.all()

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle(title: "Blobs")
            SettingsGroup {
                Toggle(isOn: $settings.showBlobs) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Show blobs in the sidebar")
                        Text("A row of little bots above Astrid's mark. Click one to pick it; it reacts while Astrid works.")
                            .font(.caption)
                            .foregroundStyle(palette.secondary)
                    }
                }
                .toggleStyle(.switch)
            }
            SectionTitle(title: "In the row (\(settings.blobsShown.filter { id in every.contains { $0.id == id } }.count) of \(Blobs.limit))")
            SettingsGroup {
                ForEach(every) { blob in
                    let on = settings.blobsShown.contains(blob.id)
                    let full = settings.blobsShown.filter { id in every.contains { $0.id == id } }.count >= Blobs.limit
                    Toggle(isOn: Binding(
                        get: { on },
                        set: { next in
                            if next { settings.blobsShown.append(blob.id) } else { settings.blobsShown.removeAll { $0 == blob.id } }
                        })) {
                        Text(blob.name)
                    }
                    .disabled(!on && full)
                    if blob.id != every.last?.id { Divider() }
                }
            }
            SectionTitle(title: "More blobs")
            SettingsGroup {
                Text("The built-in bots are from Bible Strong Avatar Lab (AGPL-3.0). To add your own, make one at avatars.bible-strong.app, export it (Export › .avatar.json), put it in the Blobs folder, and press Reload.")
                    .font(.caption)
                    .foregroundStyle(palette.secondary)
                HStack {
                    Button("Open Avatar Lab") { NSWorkspace.shared.open(URL(string: "https://avatars.bible-strong.app")!) }
                        .buttonStyle(.glass)
                    Button("Open Folder") {
                        try? FileManager.default.createDirectory(at: Blobs.folder, withIntermediateDirectories: true)
                        NSWorkspace.shared.open(Blobs.folder)
                    }
                    .buttonStyle(.glass)
                    Button("Reload") { every = Blobs.all(); NotificationCenter.default.post(name: Blobs.changed, object: nil) }
                        .buttonStyle(.glass)
                }
            }
        }
    }
}
