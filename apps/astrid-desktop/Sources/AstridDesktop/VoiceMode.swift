import AVFoundation
import Combine
import DesktopCore
import Foundation

/// Hands-free voice in the composer: the waveform button turns the input into
/// Astrid with her voice ring. What you say is sent when you stop talking, her
/// reply is spoken (a drawing's too), then she listens again. The microphone
/// is open only while listening, speech is recognised on this Mac only (ADR
/// 0034), and replies are spoken by the local voice or macOS's (ADR 0035).
final class VoiceMode: ObservableObject {
    enum Phase: Equatable { case off, listening, thinking, speaking }

    @Published private(set) var phase: Phase = .off
    /// Loudness of the mic or of Astrid's voice, 0...1, for the ring.
    @Published private(set) var level: Double = 0
    /// What has been heard so far, while listening.
    @Published private(set) var heard = ""
    @Published private(set) var problem: String?
    /// Mic off: she still speaks, but doesn't listen until unmuted.
    @Published private(set) var muted = false

    /// Sends what was said; returns false if it couldn't be asked.
    var ask: (String) -> Bool = { _ in false }
    /// Every analyser frame of the mic (while listening) or of her voice
    /// (while speaking), for the voice ring.
    var onFrame: ((AnalyserFrame) -> Void)?
    /// How long a pause ends your turn, as in presence's voice mode.
    static let pause: TimeInterval = 1.3
    private var pauseTimer: Timer?
    var stopAsk: () -> Void = {}

    private let input = VoiceInput()
    private lazy var speaker: Speaker = {
        let command = repoFile("apps/astrid-voice/.venv/bin/astrid-voice")
        let speaker = Speaker(command: command.flatMap { FileManager.default.isExecutableFile(atPath: $0.path) ? $0 : nil }, voice: "af_heart")
        speaker.onFrame = { [weak self] frame in self?.meter(frame) }
        speaker.onSpeaking = { [weak self] on in
            guard let self, self.phase != .off else { return }
            if on { self.phase = .speaking } else if self.phase == .speaking { self.listen() }
        }
        return speaker
    }()

    init() {
        input.onFrame = { [weak self] frame in
            guard self?.phase == .listening else { return }
            self?.meter(frame)
        }
        input.onTranscript = { [weak self] text, final in
            guard let self, self.phase == .listening else { return }
            self.heard = text
            self.pauseTimer?.invalidate()
            guard final else {
                // Stopped talking: end the turn, which settles the transcript
                // and sends it, with no button to press.
                if !text.trimmingCharacters(in: .whitespaces).isEmpty {
                    self.pauseTimer = Timer.scheduledTimer(withTimeInterval: Self.pause, repeats: false) { [weak self] _ in
                        guard let self, self.phase == .listening else { return }
                        self.input.stop()
                    }
                }
                return
            }
            let said = text.trimmingCharacters(in: .whitespacesAndNewlines)
            if said.isEmpty { return self.listen() }
            self.level = 0
            if self.ask(said) { self.phase = .thinking } else { self.listen() }
        }
        input.onError = { [weak self] message in
            self?.problem = message
            self?.end()
        }
    }

    func start() {
        guard phase == .off else { return }
        problem = nil
        muted = false
        speaker.prepare()
        listen()
    }

    /// Back to the keyboard: stops listening and speaking.
    func end() {
        pauseTimer?.invalidate()
        phase = .off
        level = 0
        heard = ""
        input.stop()
        speaker.stop()
    }

    /// Debug: shows the panel listening without opening the mic, for snapshots.
    func preview() {
        phase = .listening
        heard = "Show me the parts of the human brain"
        level = 0.5
    }

    func toggleMute() {
        muted.toggle()
        if muted {
            input.stop()
            if phase == .listening { level = 0; heard = "" }
        } else if phase == .listening {
            input.start()
        }
    }

    /// Tapping the ring while she speaks or thinks interrupts her.
    func interrupt() {
        switch phase {
        case .speaking: speaker.stop(); listen()
        case .thinking: stopAsk()
        default: break
        }
    }

    /// The reply to a voice ask is ready: say it, then listen again.
    func replied(_ message: ChatMessage) {
        guard phase == .thinking else { return }
        let words = Self.spoken(message)
        if words.isEmpty { return listen() }
        phase = .speaking
        speaker.say(words)
    }

    private func listen() {
        heard = ""
        level = 0
        phase = .listening
        if !muted { input.start() }
    }

    private func meter(_ frame: AnalyserFrame) {
        onFrame?(frame)
        let rms = sqrt(frame.time.reduce(0) { $0 + Double($1 * $1) } / Double(max(frame.time.count, 1)))
        let next = min(1, rms * 9)
        level = level * 0.6 + next * 0.4
    }

    /// What she says out loud: the gist of the reply, or what she drew.
    static func spoken(_ message: ChatMessage) -> String {
        let plain = message.text
            .replacingOccurrences(of: #"\*\*|__|`|#+\s|(?m)^\s*[-*]\s|(?m)^\s*\d+\.\s"#, with: "", options: .regularExpression)
            .replacingOccurrences(of: #"\s+"#, with: " ", options: .regularExpression)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if !plain.isEmpty {
            guard plain.count > 600 else { return plain }
            let cut = String(plain.prefix(600))
            if let end = cut.range(of: ". ", options: .backwards), cut.distance(from: cut.startIndex, to: end.lowerBound) > 300 {
                return String(cut[..<end.upperBound])
            }
            return cut
        }
        let title = message.diagram?.first { $0["shape"]?.string == "title" }?["text"]?.string
        if let title { return "Here's \(title)." }
        return message.diagram == nil ? "" : "Here's the drawing."
    }
}
