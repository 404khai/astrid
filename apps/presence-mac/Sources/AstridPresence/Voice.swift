import Accelerate
import AVFoundation
import Speech

/// V2-03 (ADR 0034): push-to-talk capture for the ask surface. The microphone
/// is opened only while the user holds (or toggles) the mic; audio is never
/// written to disk or sent anywhere. Speech is transcribed on-device only;
/// when the Mac cannot recognise on-device, listening fails rather than
/// falling back to Apple's servers.
final class VoiceInput {
    /// One analyser frame per audio buffer: the latest 1024 samples and their
    /// byte spectrum, shaped like a Web Audio AnalyserNode so the page's voice
    /// ring reads it exactly as it reads a browser microphone.
    var onFrame: (AnalyserFrame) -> Void = { _ in }
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
        let analyser = Analyser(sampleRate: format.sampleRate)
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            request.append(buffer)
            guard let frame = analyser.read(buffer) else { return }
            DispatchQueue.main.async { self?.onFrame(frame) }
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
}

struct AnalyserFrame {
    var sampleRate: Double
    var time: [Float]
    var bytes: [UInt8]

    var message: [String: Any] {
        let time = self.time.withUnsafeBufferPointer { Data(buffer: $0).base64EncodedString() }
        return ["type": "voice", "sampleRate": sampleRate, "time": time, "freq": Data(bytes).base64EncodedString()]
    }
}

/// Web Audio's AnalyserNode in miniature (fftSize 1024, Blackman window,
/// smoothing 0.5, −100…−30 dB to bytes), run on the audio tap's thread.
final class Analyser {
    static let size = 1024
    private let sampleRate: Double
    private var ring = [Float](repeating: 0, count: Analyser.size)
    private var smoothed = [Float](repeating: 0, count: Analyser.size / 2)
    private let window = vDSP.window(ofType: Float.self, usingSequence: .blackman, count: Analyser.size, isHalfWindow: false)
    private let fft = vDSP.FFT(log2n: 10, radix: .radix2, ofType: DSPSplitComplex.self)

    init(sampleRate: Double) { self.sampleRate = sampleRate }

    func read(_ buffer: AVAudioPCMBuffer) -> AnalyserFrame? {
        guard let samples = buffer.floatChannelData?[0], let fft else { return nil }
        let n = Analyser.size, count = Int(buffer.frameLength)
        let fresh = Array(UnsafeBufferPointer(start: samples, count: count).suffix(n))
        ring = Array((ring + fresh).suffix(n))

        let windowed = vDSP.multiply(ring, window)
        var real = [Float](repeating: 0, count: n / 2), imag = [Float](repeating: 0, count: n / 2)
        var magnitudes = [Float](repeating: 0, count: n / 2)
        real.withUnsafeMutableBufferPointer { re in
            imag.withUnsafeMutableBufferPointer { im in
                var split = DSPSplitComplex(realp: re.baseAddress!, imagp: im.baseAddress!)
                windowed.withUnsafeBytes { raw in
                    vDSP_ctoz(raw.bindMemory(to: DSPComplex.self).baseAddress!, 2, &split, 1, vDSP_Length(n / 2))
                }
                fft.forward(input: split, output: &split)
                vDSP.absolute(split, result: &magnitudes)
            }
        }
        var bytes = [UInt8](repeating: 0, count: n / 2)
        for i in 0..<n / 2 {
            // zrip doubles the DFT; Web Audio divides by the size.
            let magnitude = magnitudes[i] / 2 / Float(n)
            smoothed[i] = 0.5 * smoothed[i] + 0.5 * magnitude
            let db = 20 * log10(max(smoothed[i], 1e-12))
            bytes[i] = UInt8(max(0, min(255, 255 * (db + 100) / 70)))
        }
        return AnalyserFrame(sampleRate: sampleRate, time: ring, bytes: bytes)
    }
}
