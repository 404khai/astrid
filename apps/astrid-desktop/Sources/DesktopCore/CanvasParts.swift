import Foundation

/// Astrid Weaver in the desktop app: the parts of a drawing, follow-ups that
/// only edit it, and exported files.
public enum CanvasParts {
    /// The part ids a drawing names, in order, once each, so a follow-up can
    /// say "make the turbine faster" and the model can edit that part.
    public static func ids(in shapes: [JSONValue]) -> [String] {
        var seen = Set<String>()
        return shapes.compactMap { shape -> String? in
            guard shape["shape"]?.string != "edit", let id = shape["id"]?.string, seen.insert(id).inserted else { return nil }
            return id
        }
    }

    /// A reply whose drawing starts with an edit line changes the drawing
    /// before it rather than drawing a new one.
    public static func isEdit(_ shape: JSONValue) -> Bool { shape["shape"]?.string == "edit" }
}

extension ChatSession {
    /// The most recent drawing in this chat, which an edit-only reply changes.
    public var latestDiagram: [JSONValue]? {
        messages.last { $0.role == .astrid && !($0.diagram ?? []).isEmpty }?.diagram
    }
}

/// Files the canvas page exports: SVG text or base64 PNG bytes.
public enum CanvasExport {
    public enum Format: String, Sendable { case svg, png }

    /// The bytes to save, or nil if the page sent something that isn't the
    /// format it claims.
    public static func data(format: Format, payload: String) -> Data? {
        switch format {
        case .svg:
            let text = payload.trimmingCharacters(in: .whitespacesAndNewlines)
            guard text.hasPrefix("<svg"), let data = text.data(using: .utf8) else { return nil }
            return data
        case .png:
            guard let data = Data(base64Encoded: payload), data.starts(with: [0x89, 0x50, 0x4E, 0x47]) else { return nil }
            return data
        }
    }

    /// A file name with no path parts, ending in the format's extension.
    public static func fileName(_ name: String, format: Format) -> String {
        let base = (name as NSString).deletingPathExtension
            .components(separatedBy: CharacterSet(charactersIn: "/\\:")).joined(separator: " ")
            // No leading dots, so the file is never hidden or relative.
            .trimmingCharacters(in: CharacterSet.whitespacesAndNewlines.union(CharacterSet(charactersIn: ".")))
        return "\(base.isEmpty ? "Astrid drawing" : String(base.prefix(80))).\(format.rawValue)"
    }
}
