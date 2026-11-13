import AppKit
import PresenceCore

let usage = """
astrid-presence: Astrid's macOS presence overlay (V2-01). Draws Astrid's cursor,
ink and subtitles from a scripted presence-protocol v1 stream. No model, no screen
capture, no input injection, no network listener.

USAGE
  astrid-presence [--demo | --script FILE | --stdin] [--reduce-motion] [--exit-at-end]
  astrid-presence --print-demo

OPTIONS
  --demo           Play the bundled tour for the connected displays (default).
  --script FILE    Play a JSON-lines script. Every line is validated first.
  --stdin          Read JSON-lines events from standard input as they arrive.
  --print-demo     Print the demo for the current displays as JSON lines and exit.
  --reduce-motion  Behave as if Reduce Motion were on (it is also followed live).
  --exit-at-end    Quit after the input ends and the queue drains.
  --capture-source screen|synthetic
                   Where `capture` events read frames (default screen, which needs
                   Screen Recording permission). synthetic draws a fake desktop.
  --resident       Stay in the menu bar and own timers/alarms (ADR 0029), saved in
                   ~/Library/Application Support/Astrid/timers.json.
  --timers-file PATH  Use another timers file (testing).
  --ask            Show only the Astrid mark in the bottom-right corner; click it to
                   ask (ADR 0033). Answers come from `astrid presence-ask`.
  --ask-model M    Model for --ask (default claude-code/sonnet).
  --ask-mail SRC   gmail, apple, all or none (default gmail).
  --astrid PATH    The astrid binary (default: target/debug/astrid in the repo).
  --ask-ui PATH    The built ask UI (default: web/dist-presence/presence.html).
  --snapshot DIR   Debug: once a second, write PNGs of Astrid's own overlay layers
                   (never the screen beneath) to DIR.
  -h, --help       Show this help.

Quit with Ctrl-C, {"v":1,"type":"quit"}, or the Astrid menu-bar icon.
Protocol: docs/adr/0026-presence-host-and-protocol.md
"""

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("astrid-presence: \(message)\n\nRun with --help for usage.\n".utf8))
    exit(2)
}

func currentLayout() -> DisplayLayout {
    let screens = NSScreen.screens
    return DisplayLayout.fromCocoa(frames: screens.map(\.frame), scales: screens.map { Double($0.backingScaleFactor) })
}

var source: PresenceController.Source = .demo
var reduceMotion = false
var exitAtEnd = false
var resident = false
var timersURL = TimerStore.defaultURL
var snapshotDirectory: URL?
var captureSource = CaptureSource.screen
var askMode = false
var askModel = "claude-code/sonnet"
var askMail: String? = "gmail"
var astridPath: String?
var askUIPath: String?
var arguments = CommandLine.arguments.dropFirst()
var sourceCount = 0

while let argument = arguments.popFirst() {
    switch argument {
    case "-h", "--help":
        print(usage)
        exit(0)
    case "--demo":
        source = .demo
        sourceCount += 1
    case "--stdin":
        source = .stdin
        sourceCount += 1
    case "--script":
        guard let path = arguments.popFirst() else { fail("--script needs a file path") }
        let url = URL(fileURLWithPath: path)
        guard let attributes = try? FileManager.default.attributesOfItem(atPath: url.path),
              let size = attributes[.size] as? Int else { fail("cannot read \(path)") }
        guard size <= PresenceProtocol.Limits.maxScriptBytes else {
            fail("\(path) exceeds \(PresenceProtocol.Limits.maxScriptBytes) bytes")
        }
        guard let text = try? String(contentsOf: url, encoding: .utf8) else { fail("\(path) is not UTF-8 text") }
        let script = PresenceProtocol.parseScript(text)
        if !script.errors.isEmpty {
            for (line, error) in script.errors {
                FileHandle.standardError.write(Data("\(path):\(line): \(error.message)\n".utf8))
            }
            fail("\(script.errors.count) invalid line(s); nothing was played")
        }
        source = .script(script.events.map(\.event))
        sourceCount += 1
    case "--print-demo":
        print("# Astrid presence demo, protocol v1, generated for the current displays.")
        for event in DemoScript.events(for: currentLayout()) { print(PresenceProtocol.encode(event)) }
        exit(0)
    case "--reduce-motion":
        reduceMotion = true
    case "--exit-at-end":
        exitAtEnd = true
    case "--capture-source":
        guard let value = arguments.popFirst(), let parsed = CaptureSource(rawValue: value) else {
            fail("--capture-source needs screen or synthetic")
        }
        captureSource = parsed
    case "--resident":
        resident = true
    case "--timers-file":
        guard let path = arguments.popFirst() else { fail("--timers-file needs a path") }
        timersURL = URL(fileURLWithPath: path)
    case "--ask":
        askMode = true
    case "--ask-model":
        guard let value = arguments.popFirst() else { fail("--ask-model needs a model") }
        askModel = value
    case "--ask-mail":
        guard let value = arguments.popFirst(), ["gmail", "apple", "all", "none"].contains(value) else { fail("--ask-mail needs gmail, apple, all or none") }
        askMail = value == "none" ? nil : value
    case "--astrid":
        guard let value = arguments.popFirst() else { fail("--astrid needs a path") }
        astridPath = value
    case "--ask-ui":
        guard let value = arguments.popFirst() else { fail("--ask-ui needs a path") }
        askUIPath = value
    case "--snapshot":
        guard let path = arguments.popFirst() else { fail("--snapshot needs a directory") }
        let url = URL(fileURLWithPath: path, isDirectory: true)
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        snapshotDirectory = url
    default:
        fail("unknown argument \(argument)")
    }
}
if sourceCount > 1 { fail("choose one of --demo, --script or --stdin") }

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

var askConfig: AskConfig?
if askMode {
    guard let ui = askUIPath.map({ URL(fileURLWithPath: $0) }) ?? repoFile("web/dist-presence/presence.html") else {
        fail("ask UI not built: run `npx vite build -c vite.presence.config.ts` in web/, or pass --ask-ui")
    }
    guard let astrid = astridPath.map({ URL(fileURLWithPath: $0) }) ?? repoFile("target/debug/astrid") else {
        fail("astrid binary not found: run `cargo build` in the repo, or pass --astrid")
    }
    askConfig = AskConfig(ui: ui, astrid: astrid, model: askModel, mail: askMail)
    // The ask surface replaces the scripted demo; Astrid waits in the corner.
    if sourceCount == 0 { source = .script([]) }
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let controller = PresenceController(source: source, forceReduceMotion: reduceMotion, exitAtEnd: exitAtEnd, snapshotDirectory: snapshotDirectory, captureSource: captureSource, resident: resident, timersURL: timersURL, ask: askConfig)
app.delegate = controller
app.run()
