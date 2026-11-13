import Foundation

/// One line from `astrid presence-ask` (ADRs 0033/0036), as the desktop app uses it.
public enum AskEvent: Equatable {
    case ready
    case state(state: String, label: String)
    case delta(String)
    case approval(id: Int, title: String)
    case answer(text: String, brief: JSONValue?, drew: Bool)
    case canvasBegin
    case canvasShape(JSONValue)
    case canvasEnd
    case canvasCancel
    case error(String)
    case done(outcome: String)

    /// Parses one stdout line; nil for anything malformed or not used here.
    public static func parse(_ line: Data) -> AskEvent? {
        guard let object = try? JSONSerialization.jsonObject(with: line) as? [String: Any],
              let type = object["type"] as? String else { return nil }
        switch type {
        case "ready": return .ready
        case "state":
            guard let state = object["state"] as? String else { return nil }
            return .state(state: state, label: object["label"] as? String ?? "")
        case "delta":
            guard let text = object["text"] as? String else { return nil }
            return .delta(text)
        case "approval":
            guard let id = object["id"] as? Int else { return nil }
            return .approval(id: id, title: object["title"] as? String ?? "Allow this step?")
        case "answer":
            let brief = (object["brief"] as? [String: Any]).flatMap(JSONValue.init(any:))
            return .answer(text: object["text"] as? String ?? "", brief: brief, drew: object["canvas"] as? Bool ?? false)
        case "canvas":
            switch object["event"] as? String {
            case "begin": return .canvasBegin
            case "shape":
                guard let shape = (object["shape"] as? [String: Any]).flatMap(JSONValue.init(any:)) else { return nil }
                return .canvasShape(shape)
            case "end": return .canvasEnd
            case "cancel": return .canvasCancel
            default: return nil
            }
        case "error":
            return .error(object["message"] as? String ?? "Something went wrong.")
        case "done":
            return .done(outcome: object["outcome"] as? String ?? "failed")
        default: return nil
        }
    }
}

/// Splits a byte stream into lines, dropping everything if a line grows past
/// `limit` without ending (a broken child must not grow memory without bound).
public struct LineBuffer {
    private var buffer = Data()
    private let limit: Int

    public init(limit: Int = 8 * 1024 * 1024) { self.limit = limit }

    public mutating func append(_ chunk: Data) -> [Data] {
        buffer.append(chunk)
        var lines: [Data] = []
        while let newline = buffer.firstIndex(of: UInt8(ascii: "\n")) {
            lines.append(Data(buffer[buffer.startIndex..<newline]))
            buffer.removeSubrange(buffer.startIndex...newline)
        }
        if buffer.count > limit { buffer.removeAll() }
        return lines
    }
}

/// The desktop app's own `astrid presence-ask` child. It is started on the
/// first ask and belongs to this app only: the presence app runs its own, and
/// nothing here starts, signals or waits on the presence app.
public final class AskChild {
    public struct Config {
        public var astrid: URL
        public var model: String
        public var mail: String?
        public init(astrid: URL, model: String, mail: String?) {
            self.astrid = astrid
            self.model = model
            self.mail = mail
        }
    }

    private let config: Config
    private var process: Process?
    private var input: FileHandle?
    private var lines = LineBuffer()
    /// Called on the main queue for each event, and with `nil` when the child stops.
    public var onEvent: (AskEvent?) -> Void = { _ in }

    public init(config: Config) { self.config = config }

    public var arguments: [String] {
        var arguments = ["presence-ask", "--model", config.model]
        if let mail = config.mail { arguments += ["--mail-source", mail] }
        return arguments
    }

    /// Sends one message, starting the child if it isn't running.
    @discardableResult
    public func send(_ message: [String: Any]) -> Bool {
        if process?.isRunning != true, !start() { return false }
        guard let input, let data = try? JSONSerialization.data(withJSONObject: message) else { return false }
        do {
            try input.write(contentsOf: data + Data("\n".utf8))
            return true
        } catch {
            return false
        }
    }

    public func ask(_ text: String, history: [[String: String]]) -> Bool {
        send(["type": "ask", "text": text, "history": history])
    }

    private func start() -> Bool {
        let process = Process()
        process.executableURL = config.astrid
        process.arguments = arguments
        let stdin = Pipe(), stdout = Pipe()
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = FileHandle.nullDevice
        lines = LineBuffer()
        stdout.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let chunk = handle.availableData
            DispatchQueue.main.async { self?.receive(chunk) }
        }
        process.terminationHandler = { [weak self] ended in
            DispatchQueue.main.async {
                guard let self, self.process === ended else { return }
                self.process = nil
                self.input = nil
                self.onEvent(nil)
            }
        }
        do {
            try process.run()
        } catch {
            return false
        }
        self.process = process
        input = stdin.fileHandleForWriting
        return true
    }

    private func receive(_ chunk: Data) {
        guard !chunk.isEmpty else { return }
        for line in lines.append(chunk) {
            if let event = AskEvent.parse(line) { onEvent(event) }
        }
    }

    /// Closes the child's stdin (it cancels any running ask and exits) and
    /// terminates it if it is still there.
    public func stop() {
        guard let process else { return }
        self.process = nil
        try? input?.close()
        input = nil
        if process.isRunning { process.terminate() }
    }
}
