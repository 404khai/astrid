import AVFoundation
import Speech

/// V2-03 (ADR 0034): push-to-talk capture for the ask surface. The microphone
/// is opened only while the user holds (or toggles) the mic; audio is never
/// written to disk or sent anywhere. Speech is transcribed on-device only;
/// when the Mac cannot recognise on-device, listening fails rather than
/// falling back to Apple's servers.
final class VoiceInput {
    /// Shaped loudness 0–1, at the audio tap's rate (~20–50 Hz).
    var onLevel: (Double) -> Void = { _ in }
    /// The best transcript so far; `final` once recognition has settled.
    var onTranscript: (String, Bool) -> Void = { _, _ in }
    var onError: (String) -> Void = { _ in }

    private let engine = AVAudioEngine()
    private var request: SFSpeechAudioBufferRecognitionRequest?
    private var task: SFSpeechRecognitionTask?
    private var recognizer: SFSpeechRecognizer?
    private var running = false
    private var latest = ""

    func start() {
        guard !running else { return }
        authorize { [weak self] message in
            guard let self else { return }
            if let message { return self.onError(message) }
            do { try self.begin() } catch { self.onError("Could not open the microphone: \(error.localizedDescription)") }
        }
    }

    /// Stops the microphone; recognition finishes and reports the final text.
    func stop() {
        guard running else { return }
        running = false
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        request?.endAudio()
        onLevel(0)
    }

    private func authorize(_ done: @escaping (String?) -> Void) {
        AVCaptureDevice.requestAccess(for: .audio) { granted in
            guard granted else {
                return DispatchQueue.main.async { done("Microphone access is off for Astrid Presence (System Settings → Privacy & Security → Microphone).") }
            }
            SFSpeechRecognizer.requestAuthorization { status in
                DispatchQueue.main.async {
                    done(status == .authorized ? nil : "Speech recognition is off for Astrid Presence (System Settings → Privacy & Security → Speech Recognition).")
                }
            }
        }
    }

    private func begin() throws {
        guard let recognizer = SFSpeechRecognizer(locale: Locale.current) ?? SFSpeechRecognizer(locale: Locale(identifier: "en-US")),
              recognizer.supportsOnDeviceRecognition else {
            return onError("On-device speech recognition isn't available for this language on this Mac.")
        }
        let request = SFSpeechAudioBufferRecognitionRequest()
        request.requiresOnDeviceRecognition = true
        request.shouldReportPartialResults = true
        self.recognizer = recognizer
        self.request = request
        latest = ""

        let input = engine.inputNode
        let format = input.outputFormat(forBus: 0)
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            request.append(buffer)
            let level = VoiceInput.level(of: buffer)
            DispatchQueue.main.async { self?.onLevel(level) }
        }
        task = recognizer.recognitionTask(with: request) { [weak self] result, error in
            DispatchQueue.main.async {
                guard let self else { return }
                if let result {
                    self.latest = result.bestTranscription.formattedString
                    self.onTranscript(self.latest, result.isFinal)
                    if result.isFinal { self.finish() }
                } else if error != nil {
                    // Ending with no speech reports an error; deliver what we have.
                    self.onTranscript(self.latest, true)
                    self.finish()
                }
            }
        }
        engine.prepare()
        try engine.start()
        running = true
    }

    private func finish() {
        if running { stop() }
        task = nil
        request = nil
    }

    /// RMS of the first channel mapped from −50…0 dBFS onto 0…1.
    static func level(of buffer: AVAudioPCMBuffer) -> Double {
        guard let samples = buffer.floatChannelData?[0], buffer.frameLength > 0 else { return 0 }
        var sum: Float = 0
        for i in 0..<Int(buffer.frameLength) { sum += samples[i] * samples[i] }
        let rms = sqrt(sum / Float(buffer.frameLength))
        let db = 20 * log10(max(rms, 1e-7))
        return Double(min(max((db + 50) / 50, 0), 1))
    }
}
