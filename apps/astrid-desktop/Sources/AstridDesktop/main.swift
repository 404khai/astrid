import AppKit
import DesktopCore

let usage = """
astrid-desktop: Astrid's desktop chat app (ADR 0037). A normal Dock app with a
sidebar of chats and the selected chat on the right. Answers come from its own
`astrid presence-ask` child; Astrid Presence keeps running on its own.

USAGE
  astrid-desktop [--model M] [--mail SRC] [--astrid PATH] [--ui DIR] [--chats DIR]

OPTIONS
  --model M      Model for asks, over the one in Settings (default claude-code/sonnet).
  --mail SRC     gmail, apple, all or none, over Settings (default gmail). Mail stays
                 read-only, with an Allow once card for every mail call (ADRs 0028/0033).
  --astrid PATH  The astrid binary (default: target/debug/astrid in the repo).
  --ui DIR       The built web UI (default: web/dist-presence in the repo).
  --chats DIR    Where chats are kept (default
                 ~/Library/Application Support/Astrid/Chats).
  -h, --help     Show this help.

Debug: ASTRID_DESKTOP_SNAPSHOT=/path.png saves the app's own window content
(never the screen) a few seconds after launch.
"""

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("astrid-desktop: \(message)\n\nRun with --help for usage.\n".utf8))
    exit(2)
}

/// Walks up from this executable to the repository that contains `relative`.
func repoFile(_ relative: String) -> URL? {
    var dir = URL(fileURLWithPath: CommandLine.arguments[0]).resolvingSymlinksInPath().deletingLastPathComponent()
    for _ in 0..<10 {
        let candidate = dir.appendingPathComponent(relative)
        if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
        dir.deleteLastPathComponent()
    }
    return nil
}

var model: String?
var mail: String?
var astridPath: String?
var uiPath: String?
var chatsPath: String?
var arguments = CommandLine.arguments.dropFirst()
while let argument = arguments.popFirst() {
    switch argument {
    case "-h", "--help":
        print(usage)
        exit(0)
    case "--model":
        guard let value = arguments.popFirst() else { fail("--model needs a model") }
        model = value
    case "--mail":
        guard let value = arguments.popFirst(), ["gmail", "apple", "all", "none"].contains(value) else { fail("--mail needs gmail, apple, all or none") }
        mail = value
    case "--astrid":
        guard let value = arguments.popFirst() else { fail("--astrid needs a path") }
        astridPath = value
    case "--ui":
        guard let value = arguments.popFirst() else { fail("--ui needs a folder") }
        uiPath = value
    case "--chats":
        guard let value = arguments.popFirst() else { fail("--chats needs a folder") }
        chatsPath = value
    default:
        // LaunchServices may pass -psn_… and -NS… arguments to a bundled app.
        if argument.hasPrefix("-psn_") || argument.hasPrefix("-NS") || argument.hasPrefix("-Apple") {
            _ = argument.hasPrefix("-psn_") ? nil : arguments.popFirst()
            continue
        }
        fail("unknown argument \(argument)")
    }
}

guard let ui = uiPath.map({ URL(fileURLWithPath: $0, isDirectory: true) }) ?? repoFile("web/dist-presence") else {
    fail("web UI not built: run `npx vite build -c vite.presence.config.ts` in web/, or pass --ui")
}
let astrid = astridPath.map { URL(fileURLWithPath: $0) } ?? repoFile("target/debug/astrid") ?? URL(fileURLWithPath: "/usr/local/bin/astrid")
let configuration = DesktopConfig(
    ui: ui,
    astrid: astrid,
    chats: chatsPath.map { URL(fileURLWithPath: $0, isDirectory: true) } ?? ChatStore.defaultDirectory,
    modelOverride: model,
    mailOverride: mail
)

DesktopApp.configuration = configuration
DesktopApp.main()
