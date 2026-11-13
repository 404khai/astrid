import AVFoundation

/// V2-03 (ADR 0035): Astrid's spoken replies. Text goes to the local
/// `astrid-voice` process (Kokoro-82M on MLX), which streams PCM back one
/// sentence at a time; without it, macOS's built-in voice speaks instead.
/// Playback runs through the same analyser as the mic so the voice ring moves
/// with Astrid's voice. Nothing is sent off the Mac and no audio is saved.
final class Speaker {
    var onFrame: (AnalyserFrame) -> Void = { _ in }
    var onSpeaking: (Bool) -> Void = { _ in }

    private let command: URL?
    private let voice: String
    private let engine = AVAudioEngine()
    private let player = AVAudioPlayerNode()
    private let format = AVAudioFormat(standardFormatWithSampleRate: 24_000, channels: 1)!
    private var child: Process?
    private var childInput: FileHandle?
    private var buffer = Data()
    private var synthesizer: AVSpeechSynthesizer?

    /// The utterance being spoken; audio for any other id is dropped.
    private var current = 0
    private var pending = 0
    private var finishedInput = true
    private var metrics = SpeechMetrics()

    init(command: URL?, voice: String) {
        self.command = command
        self.voice = voice
        engine.attach(player)
        engine.connect(player, to: engine.mainMixerNode, format: format)
        let analyser = Analyser(sampleRate: format.sampleRate)
        player.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            guard let frame = analyser.read(buffer) else { return }
            DispatchQueue.main.async {
                guard let self, self.pending > 0 else { return }
                self.onFrame(frame)
            }
        }
    }

    func say(_ text: String) {
        let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return onSpeaking(false) }
        stop()
        current += 1
        finishedInput = false
        metrics = SpeechMetrics(chars: text.count)
        do {
            if !engine.isRunning { try engine.start() }
        } catch {
            Speaker.note("playback engine failed to start: \(error.localizedDescription)")
            finishedInput = true
            return onSpeaking(false)
        }
        Speaker.note("say #\(current), \(text.count) chars")
        player.play()
        onSpeaking(true)
        if let input = sidecar() {
            metrics.engine = "kokoro"
            let line = ["type": "say", "id": current, "text": text, "voice": voice] as [String: Any]
            guard let data = try? JSONSerialization.data(withJSONObject: line) else { return }
            try? input.write(contentsOf: data + Data("\n".utf8))
        } else {
            metrics.engine = "macos"
            speakBuiltIn(text, id: current)
        }
    }

    /// Starts the Kokoro process ahead of the first reply (it takes a few
    /// seconds to load), so it is warm by the time an answer arrives.
    func prepare() {
        _ = sidecar()
    }

    /// Barge-in: silence now and drop anything still arriving.
    func stop() {
        guard !finishedInput || pending > 0 else { return }
        Speaker.note("stop #\(current) with \(pending) buffers queued")
        if let input = childInput, let data = try? JSONSerialization.data(withJSONObject: ["type": "stop"]) {
            try? input.write(contentsOf: data + Data("\n".utf8))
        }
        synthesizer?.stopSpeaking(at: .immediate)
        current += 1
        player.stop()
        metrics.interrupted = true
        finish()
    }

    func shutdown() {
        stop()
        child?.terminate()
    }

    // MARK: Playback

    private func play(_ samples: [Float], id: Int) {
        guard id == current, !samples.isEmpty,
              let pcm = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(samples.count)) else {
            return Speaker.note("dropped audio for #\(id) (current #\(current), \(samples.count) samples)")
        }
        // The output device can change under a running engine (headphones, the
        // mic engine starting); restart it rather than scheduling into silence.
        if !engine.isRunning { try? engine.start() }
        if !player.isPlaying { player.play() }
        pcm.frameLength = pcm.frameCapacity
        samples.withUnsafeBufferPointer { pcm.floatChannelData![0].update(from: $0.baseAddress!, count: samples.count) }
        metrics.firstAudio()
        pending += 1
        player.scheduleBuffer(pcm, completionCallbackType: .dataPlayedBack) { [weak self] _ in
            DispatchQueue.main.async {
                guard let self, id == self.current else { return }
                self.pending -= 1
                if self.pending == 0 && self.finishedInput { self.finish() }
            }
        }
    }

    private func inputDone(id: Int) {
        guard id == current else { return }
        Speaker.note("synthesis done #\(id), \(pending) buffers queued")
        finishedInput = true
        metrics.synthesized()
        if pending == 0 { finish() }
    }

    private func finish() {
        finishedInput = true
        pending = 0
        metrics.record()
        onSpeaking(false)
    }

    // MARK: Built-in fallback

    private func speakBuiltIn(_ text: String, id: Int) {
        let synthesizer = self.synthesizer ?? AVSpeechSynthesizer()
        self.synthesizer = synthesizer
        let utterance = AVSpeechUtterance(string: text)
        let converterTarget = format
        var converter: AVAudioConverter?
        synthesizer.write(utterance) { [weak self] buffer in
            guard let pcm = buffer as? AVAudioPCMBuffer else { return }
            if pcm.frameLength == 0 {
                DispatchQueue.main.async { self?.inputDone(id: id) }
                return
            }
            if converter == nil { converter = AVAudioConverter(from: pcm.format, to: converterTarget) }
            guard let converter, let samples = Speaker.convert(pcm, with: converter, to: converterTarget) else { return }
            DispatchQueue.main.async { self?.play(samples, id: id) }
        }
    }

    private static func convert(_ input: AVAudioPCMBuffer, with converter: AVAudioConverter, to format: AVAudioFormat) -> [Float]? {
        let ratio = format.sampleRate / input.format.sampleRate
        guard let output = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(Double(input.frameLength) * ratio) + 64) else { return nil }
        var fed = false
        converter.convert(to: output, error: nil) { _, status in
            if fed { status.pointee = .noDataNow; return nil }
            fed = true
            status.pointee = .haveData
            return input
        }
        guard let channel = output.floatChannelData?[0] else { return nil }
        return Array(UnsafeBufferPointer(start: channel, count: Int(output.frameLength)))
    }

    // MARK: astrid-voice process

    /// The Kokoro process, started on first use; nil when it isn't installed or failed.
    private func sidecar() -> FileHandle? {
        if child?.isRunning == true { return childInput }
        guard let command, FileManager.default.isExecutableFile(atPath: command.path) else { return nil }
        let process = Process()
        process.executableURL = command
        let input = Pipe(), output = Pipe()
        process.standardInput = input
        process.standardOutput = output
        // The process's own diagnostics (model loading, warnings), never reply text.
        process.standardError = Speaker.diagnostics() ?? FileHandle.nullDevice
        output.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let chunk = handle.availableData
            DispatchQueue.main.async { self?.receive(chunk) }
        }
        process.terminationHandler = { [weak self] process in
            Speaker.note("astrid-voice exited with status \(process.terminationStatus)")
            DispatchQueue.main.async {
                guard let self else { return }
                self.child = nil
                self.childInput = nil
                if !self.finishedInput { self.inputDone(id: self.current) }
            }
        }
        do {
            try process.run()
        } catch {
            return nil
        }
        child = process
        childInput = input.fileHandleForWriting
        return childInput
    }

    private static let logURL = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Logs/Astrid/astrid-voice.log")

    private static func diagnostics() -> FileHandle? {
        try? FileManager.default.createDirectory(at: logURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        // Append-only, so the process's output and the host's notes interleave.
        let fd = open(logURL.path, O_WRONLY | O_CREAT | O_TRUNC | O_APPEND, 0o644)
        return fd < 0 ? nil : FileHandle(fileDescriptor: fd, closeOnDealloc: true)
    }

    static func note(_ text: String) {
        guard let handle = try? FileHandle(forWritingTo: logURL) else { return }
        defer { try? handle.close() }
        _ = try? handle.seekToEnd()
        try? handle.write(contentsOf: Data("[presence] \(text)\n".utf8))
    }

    private func receive(_ chunk: Data) {
        guard !chunk.isEmpty else { return }
        buffer.append(chunk)
        if buffer.count > 64 * 1024 * 1024 { buffer.removeAll() }
        while let newline = buffer.firstIndex(of: UInt8(ascii: "\n")) {
            let line = buffer[buffer.startIndex..<newline]
            buffer.removeSubrange(buffer.startIndex...newline)
            guard let message = try? JSONSerialization.jsonObject(with: line) as? [String: Any],
                  let type = message["type"] as? String else { continue }
            let id = message["id"] as? Int ?? -1
            switch type {
            case "audio":
                guard let encoded = message["pcm"] as? String, let data = Data(base64Encoded: encoded) else { continue }
                let samples = data.withUnsafeBytes { Array($0.bindMemory(to: Float.self)) }
                play(samples, id: id)
            case "done":
                inputDone(id: id)
            case "error":
                // Fall back to the built-in voice for this reply.
                if id == current, pending == 0, let text = message["text"] as? String {
                    metrics.engine = "macos"
                    speakBuiltIn(text, id: id)
                } else {
                    inputDone(id: id)
                }
            default:
                break
            }
        }
    }
}

/// Metadata-only timing for one spoken reply (ADR 0035, like ADR 0009 traces):
/// engine, size and latency spans, never the text or audio. One JSON line per
/// reply in ~/Library/Logs/Astrid/speech.jsonl.
struct SpeechMetrics {
    var engine = "none"
    var chars = 0
    var interrupted = false
    private let started = Date()
    private var firstAudioAt: Date?
    private var synthesizedAt: Date?
    private var recorded = false

    init(chars: Int = 0) { self.chars = chars }

    mutating func firstAudio() { if firstAudioAt == nil { firstAudioAt = Date() } }
    mutating func synthesized() { if synthesizedAt == nil { synthesizedAt = Date() } }

    mutating func record() {
        guard !recorded, chars > 0 else { return }
        recorded = true
        let started = self.started
        let ms = { (date: Date?) -> Any in date.map { Int($0.timeIntervalSince(started) * 1000) } ?? NSNull() }
        let line: [String: Any] = [
            "at": ISO8601DateFormatter().string(from: started),
            "engine": engine,
            "chars": chars,
            "first_audio_ms": ms(firstAudioAt),
            "synthesis_ms": ms(synthesizedAt),
            "total_ms": ms(Date()),
            "interrupted": interrupted,
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: line, options: [.sortedKeys]) else { return }
        let dir = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Logs/Astrid")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let file = dir.appendingPathComponent("speech.jsonl")
        if !FileManager.default.fileExists(atPath: file.path) { FileManager.default.createFile(atPath: file.path, contents: nil) }
        guard let handle = try? FileHandle(forWritingTo: file) else { return }
        defer { try? handle.close() }
        _ = try? handle.seekToEnd()
        try? handle.write(contentsOf: data + Data("\n".utf8))
    }
}
