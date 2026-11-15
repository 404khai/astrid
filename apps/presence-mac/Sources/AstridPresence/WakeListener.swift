import AVFoundation
import PresenceCore
import Speech

/// ADR 0034 amendment: "hey Astrid", "yo Astrid" or "Astrid" starts a voice
/// ask hands-free. Opt-in (`--wake`). While on, the microphone stays open and
/// speech is recognised on this Mac only (`requiresOnDeviceRecognition`; no
/// fallback to Apple's servers). Audio and transcripts are never stored or
/// sent: only the words after the wake phrase leave this listener, as the ask.
/// It pauses whenever push-to-talk or Astrid's own voice is using the audio.
final class WakeListener {
    /// The words said after the wake phrase ("" for just the name).
    var onWake: (String) -> Void = { _ in }
    var onError: (String) -> Void = { _ in }

    private let engine = AVAudioEngine()
    private var request: SFSpeechAudioBufferRecognitionRequest?
    private var task: SFSpeechRecognitionTask?
    private var running = false
    private var paused = 0
    private var wanted = false
    /// Characters of the current transcript already matched, so one hit
    /// fires once however many partial results repeat it.
    private var consumed = 0
    private var pending: WakeWord.Hit?
    private var settle: Timer?
    private var renew: Timer?
    /// After the name, how long a pause ends the ask.
    private static let pause: TimeInterval = 1.2
    /// On-device recognition tasks are restarted before Speech's one-minute limit.
    private static let renewal: TimeInterval = 50

    func start() {
        wanted = true
        AVCaptureDevice.requestAccess(for: .audio) { granted in
            SFSpeechRecognizer.requestAuthorization { status in
                DispatchQueue.main.async {
                    guard granted, status == .authorized else {
                        return self.onError("Wake word needs Microphone and Speech Recognition access for Astrid Presence.")
                    }
                    self.begin()
                }
            }
        }
    }

    /// Nested: each pause needs a resume (push-to-talk and speech both pause).
    func pause() {
        paused += 1
        end()
    }

    func resume() {
        paused = max(paused - 1, 0)
        if paused == 0 { begin() }
    }

    private func begin() {
        guard wanted, paused == 0, !running else { return }
        guard let recognizer = SFSpeechRecognizer(locale: Locale.current) ?? SFSpeechRecognizer(locale: Locale(identifier: "en-US")),
              recognizer.supportsOnDeviceRecognition else {
            wanted = false
            return onError("On-device speech recognition isn't available here, so the wake word is off.")
        }
        let request = SFSpeechAudioBufferRecognitionRequest()
        request.requiresOnDeviceRecognition = true
        request.shouldReportPartialResults = true
        request.contextualStrings = ["Astrid", "hey Astrid", "yo Astrid"]
        self.request = request
        consumed = 0
        let input = engine.inputNode
        input.installTap(onBus: 0, bufferSize: 1024, format: input.outputFormat(forBus: 0)) { buffer, _ in
            request.append(buffer)
        }
        task = recognizer.recognitionTask(with: request) { [weak self] result, error in
            DispatchQueue.main.async {
                guard let self, self.request === request else { return }
                if let result { self.heard(result.bestTranscription.formattedString) }
                if error != nil || result?.isFinal == true { self.restart() }
            }
        }
        do {
            engine.prepare()
            try engine.start()
            running = true
        } catch {
            end()
            return onError("Could not open the microphone for the wake word: \(error.localizedDescription)")
        }
        renew = Timer.scheduledTimer(withTimeInterval: Self.renewal, repeats: false) { [weak self] _ in self?.restart() }
    }

    private func heard(_ transcript: String) {
        // Re-matched on every partial, so the words after the name keep growing
        // until the speaker pauses.
        if let hit = WakeWord.match(transcript, from: consumed) {
            pending = hit
            settle?.invalidate()
            settle = Timer.scheduledTimer(withTimeInterval: Self.pause, repeats: false) { [weak self] _ in self?.fire() }
        }
    }

    private func fire() {
        guard let hit = pending else { return }
        pending = nil
        consumed = hit.end
        onWake(hit.rest)
    }

    private func restart() {
        // A hit waiting for its pause still fires; the new task starts clean.
        if pending != nil { fire() }
        end()
        begin()
    }

    private func end() {
        settle?.invalidate()
        renew?.invalidate()
        guard running || request != nil else { return }
        running = false
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        let old = request
        request = nil
        old?.endAudio()
        task?.cancel()
        task = nil
    }

    func stop() {
        wanted = false
        pending = nil
        end()
    }
}
