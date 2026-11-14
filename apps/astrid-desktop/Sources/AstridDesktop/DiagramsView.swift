import DesktopCore
import SwiftUI

/// Every drawing Astrid has made, across all chats, as a grid of saved
/// diagrams. Clicking one opens it in the diagram modal.
struct DiagramsView: View {
    @ObservedObject var model: ChatModel
    @Environment(\.palette) private var palette

    var body: some View {
        let items = model.diagrams
        ScrollView {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 250, maximum: 420), spacing: 18)], spacing: 22) {
                ForEach(items) { item in
                    DiagramTile(ui: model.config.ui, item: item) { model.openDiagram = item }
                }
            }
            .padding(.horizontal, 24)
            .padding(.top, 8)
            .padding(.bottom, 24)
        }
        .overlay {
            if items.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "scribble.variable")
                        .font(.system(size: 34, weight: .light))
                        .foregroundStyle(palette.secondary)
                    Text("No diagrams yet").font(.title3.weight(.semibold))
                    Text("When Astrid draws in a chat, the drawing shows up here.")
                        .foregroundStyle(palette.secondary)
                }
                .padding(.bottom, 40)
            }
        }
    }
}

/// One saved drawing: the canvas page replays it, with clicks passing
/// through to the tile so the whole card opens the modal.
struct DiagramTile: View {
    var ui: URL
    var item: DiagramItem
    var open: () -> Void
    @Environment(\.palette) private var palette
    @State private var hovering = false

    var body: some View {
        Button(action: open) {
            VStack(alignment: .leading, spacing: 9) {
                DiagramView(ui: ui, shapes: item.shapes, animate: false, finished: true, interactive: false)
                    .frame(height: 190)
                    .clipShape(.rect(cornerRadius: 14, style: .continuous))
                    .overlay {
                        RoundedRectangle(cornerRadius: 14, style: .continuous)
                            .strokeBorder(palette.text.opacity(hovering ? 0.22 : 0.08), lineWidth: hovering ? 1 : 0.5)
                    }
                    .shadow(color: .black.opacity(hovering ? 0.25 : 0.1), radius: hovering ? 10 : 4, y: hovering ? 4 : 1)
                VStack(alignment: .leading, spacing: 2) {
                    Text(item.title).font(.callout.weight(.semibold)).lineLimit(1)
                    Text("\(item.chatTitle) · \(item.at.formatted(.relative(presentation: .named)))")
                        .font(.caption)
                        .foregroundStyle(palette.secondary)
                        .lineLimit(1)
                }
                .padding(.horizontal, 2)
            }
            .scaleEffect(hovering ? 1.015 : 1)
            .animation(.smooth(duration: 0.18), value: hovering)
            .contentShape(.rect)
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityLabel("Open diagram \(item.title)")
    }
}

/// A drawing opened from the grid: large and interactive (drag turns it,
/// pinch zooms, double-click resets). Click outside, Esc or × closes it.
struct DiagramModal: View {
    @ObservedObject var model: ChatModel
    var item: DiagramItem
    @Environment(\.palette) private var palette

    var body: some View {
        ZStack {
            Color.black.opacity(0.45)
                .ignoresSafeArea()
                .contentShape(.rect)
                .onTapGesture { close() }
                .accessibilityHidden(true)
            VStack(spacing: 0) {
                HStack(spacing: 10) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(item.title).font(.headline).lineLimit(1)
                        Text(item.chatTitle).font(.caption).foregroundStyle(palette.secondary).lineLimit(1)
                    }
                    Spacer()
                    Button("Open Chat") {
                        model.show(chat: item.chatID)
                        close()
                    }
                    .buttonStyle(.glass)
                    GlassCircle(symbol: "xmark", help: "Close (Esc)") { close() }
                        .keyboardShortcut(.cancelAction)
                }
                .padding(.horizontal, 18)
                .frame(height: 58)
                DiagramView(ui: model.config.ui, shapes: item.shapes, animate: false, finished: true)
                    .clipShape(.rect(cornerRadius: 16, style: .continuous))
                    .padding([.horizontal, .bottom], 14)
            }
            .foregroundStyle(palette.text)
            .background(palette.content, in: .rect(cornerRadius: 22, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: 22, style: .continuous).strokeBorder(palette.text.opacity(0.1), lineWidth: 0.5)
            }
            .shadow(color: .black.opacity(0.4), radius: 30, y: 10)
            .padding(48)
            .frame(maxWidth: 1100, maxHeight: 820)
        }
    }

    private func close() {
        withAnimation(.smooth(duration: 0.25)) { model.openDiagram = nil }
    }
}
