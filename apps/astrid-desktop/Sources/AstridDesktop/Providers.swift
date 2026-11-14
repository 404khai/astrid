import AppKit
import Foundation

/// The model providers Astrid's runtime can use, with the models offered in
/// Settings › General. The runtime takes any exact model ID; these are the
/// ones known to work (ADRs 0030–0032).
struct Provider: Identifiable {
    var id: String
    var name: String
    /// Its mark in the repo's `providers/` folder.
    var icon: String
    var note: String
    var models: [String]
    /// Executables whose presence means it's set up on this Mac.
    var tools: [String] = []
    /// An environment variable that holds its key, if it uses one.
    var key: String?

    var status: String? {
        if !tools.isEmpty {
            let dirs = ["/opt/homebrew/bin", "/usr/local/bin", NSHomeDirectory() + "/.local/bin", NSHomeDirectory() + "/.npm-global/bin", NSHomeDirectory() + "/.bun/bin"]
            let found = tools.contains { tool in dirs.contains { FileManager.default.isExecutableFile(atPath: "\($0)/\(tool)") } }
            return found ? "INSTALLED" : "NOT FOUND"
        }
        if let key { return ProcessInfo.processInfo.environment[key] != nil ? "KEY SET" : "API KEY" }
        return nil
    }

    var image: NSImage? { repoFile("providers/\(icon).svg").flatMap { NSImage(contentsOf: $0) } }

    static let all: [Provider] = [
        Provider(id: "claude-code", name: "Claude Code", icon: "claude", note: "Your Claude subscription, through the Agent SDK.",
                 models: ["claude-code/sonnet", "claude-code/opus", "claude-code/haiku"], tools: ["claude"]),
        Provider(id: "claude", name: "Claude API", icon: "claude", note: "An Anthropic Console API key.",
                 models: ["claude-opus-5-5", "claude-sonnet-5-5", "claude-fable-5-1", "claude-haiku-4-5"], key: "ANTHROPIC_API_KEY"),
        Provider(id: "codex", name: "Codex", icon: "codex", note: "Your ChatGPT sign-in, through Codex.",
                 models: ["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"], tools: ["codex"]),
        Provider(id: "opencode", name: "OpenCode Go", icon: "opencode", note: "An OpenCode Go key.",
                 models: ["opencode/kimi-k3", "opencode/glm-5.3"], key: "OPENCODE_API_KEY"),
    ]

    static func of(_ model: String) -> Provider? {
        all.first { $0.models.contains(model) } ?? all.first { provider in
            switch provider.id {
            case "claude-code": model.hasPrefix("claude-code/")
            case "claude": model.hasPrefix("claude-")
            case "opencode": model.hasPrefix("opencode/")
            case "codex": model.hasPrefix("gpt-")
            default: false
            }
        }
    }
}
