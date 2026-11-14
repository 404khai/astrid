import Combine
import CoreImage
import CoreImage.CIFilterBuiltins
import DesktopCore
import Darwin
import Foundation
import Network
import SwiftUI

/// The phone link (ADR 0039): while it is on in Settings and this app is
/// running, a WebSocket listener on the local network that paired phones
/// connect to. Every frame after the hello is sealed (`LinkSession`). A phone
/// sees the chats, opens one, and asks; its asks run here, through the chat
/// model, so they are saved in the same store and show on the desktop too.
final class PhoneLink: ObservableObject {
    @Published var enabled: Bool {
        didSet {
            if !Self.debugRegistry { UserDefaults.standard.set(enabled, forKey: "phone.link") }
            enabled ? start() : stop()
        }
    }
    @Published private(set) var offer: PairingOffer?
    @Published private(set) var devices: [PairedDevice]
    /// Device IDs connected now.
    @Published private(set) var connected: Set<String> = []
    @Published private(set) var problem: String?
    @Published private(set) var hosts: [String] = []

    private let registry: DeviceRegistry
    private weak var model: ChatModel?
    private var listener: NWListener?
    private var peers: [ObjectIdentifier: Peer] = [:]
    private var updates: AnyCancellable?
    private var expiry: DispatchWorkItem?

    let macName = Host.current().localizedName ?? "Mac"

    /// Debug: ASTRID_DESKTOP_DEVICES=/path keeps paired phones in that file and
    /// leaves the stored link setting alone, for link tests.
    private static let debugRegistry = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_DEVICES"] != nil

    init(model: ChatModel) {
        let registry = ProcessInfo.processInfo.environment["ASTRID_DESKTOP_DEVICES"].map { DeviceRegistry(file: URL(fileURLWithPath: $0)) } ?? DeviceRegistry()
        self.model = model
        self.registry = registry
        devices = registry.devices
        enabled = Self.debugRegistry ? false : UserDefaults.standard.bool(forKey: "phone.link")
        updates = model.updates.sink { [weak self] update in self?.relay(update) }
        if enabled { start() }
    }

    // MARK: Pairing

    /// The QR code's text for the live pairing code.
    var pairingURL: String? {
        guard let offer, offer.isValid() else { return nil }
        return offer.url(hosts: hosts, port: LinkLimits.port, name: macName)
    }

    /// A fresh two-minute pairing code; the link turns on if it was off.
    func newOffer() {
        if !enabled { enabled = true }
        hosts = Self.localAddresses()
        let next = PairingOffer()
        offer = next
        expiry?.cancel()
        let work = DispatchWorkItem { [weak self] in
            if self?.offer == next { self?.offer = nil }
        }
        expiry = work
        DispatchQueue.main.asyncAfter(deadline: .now() + LinkLimits.pairingLifetime, execute: work)
    }

    func cancelOffer() {
        offer = nil
        expiry?.cancel()
    }

    func revoke(_ id: String) {
        try? registry.remove(id)
        devices = registry.devices
        for peer in peers.values where peer.session.device?.id == id { peer.end() }
    }

    // MARK: Listener

    private func start() {
        guard listener == nil else { return }
        hosts = Self.localAddresses()
        let parameters = NWParameters.tcp
        parameters.allowLocalEndpointReuse = true
        parameters.prohibitedInterfaceTypes = [.cellular]
        let websocket = NWProtocolWebSocket.Options()
        websocket.autoReplyPing = true
        websocket.maximumMessageSize = LinkLimits.maxInbound + 1024
        parameters.defaultProtocolStack.applicationProtocols.insert(websocket, at: 0)
        do {
            let listener = try NWListener(using: parameters, on: NWEndpoint.Port(rawValue: LinkLimits.port)!)
            listener.newConnectionHandler = { [weak self] connection in self?.accept(connection) }
            listener.stateUpdateHandler = { [weak self] state in
                switch state {
                case .failed(let error):
                    self?.problem = "The phone link couldn't listen on port \(LinkLimits.port): \(error.localizedDescription)"
                    self?.listener?.cancel()
                    self?.listener = nil
                case .ready:
                    self?.problem = nil
                default: break
                }
            }
            listener.start(queue: .main)
            self.listener = listener
        } catch {
            problem = "The phone link couldn't start: \(error.localizedDescription)"
        }
    }

    private func stop() {
        listener?.cancel()
        listener = nil
        for peer in peers.values { peer.end() }
        peers = [:]
        connected = []
        cancelOffer()
    }

    private func accept(_ connection: NWConnection) {
        // Only this Mac's own network: the link is never reachable from outside it.
        guard peers.count < LinkLimits.maxConnections, case .hostPort(let host, _) = connection.endpoint,
              LocalNetwork.isLocal("\(host)") else {
            connection.cancel()
            return
        }
        let session = LinkSession(registry: registry, macName: macName) { [weak self] peek in
            guard let self, let offer = self.offer, offer.isValid() else { return nil }
            if !peek { self.offer = nil }
            return offer.secret
        }
        let peer = Peer(connection: connection, session: session)
        let key = ObjectIdentifier(peer)
        peers[key] = peer
        session.onOpen = { [weak self] device in
            self?.connected.insert(device.id)
            self?.devices = self?.registry.devices ?? []
        }
        session.onMessage = { [weak self, weak peer] message in
            guard let self, let peer else { return }
            self.handle(message, from: peer)
        }
        peer.onEnd = { [weak self] in
            guard let self else { return }
            self.peers[key] = nil
            self.connected = Set(self.peers.values.compactMap { $0.session.device?.id })
            self.devices = self.registry.devices
        }
        peer.start()
    }

    // MARK: Requests

    private func handle(_ message: [String: Any], from peer: Peer) {
        guard let model else { return }
        switch PhoneRequest.parse(message) {
        case .sync:
            peer.session.send(PhoneMessage.chats(model.chats, busy: model.running?.chat))
        case .open(let id):
            peer.open = id
            if let chat = model.chats.first(where: { $0.id == id }), let message = PhoneMessage.chat(chat) {
                peer.session.send(message)
            } else {
                peer.session.send(PhoneMessage.refused("That chat isn't on this Mac any more."))
            }
        case .ask(let chat, let text):
            if let started = model.ask(text, in: chat, from: .phone) {
                peer.open = started.chat
                peer.asked.insert(started.reply)
                peer.session.send(PhoneMessage.started(chat: started.chat, reply: started.reply))
            } else {
                peer.session.send(PhoneMessage.refused(model.running != nil ? "Astrid is busy with another ask." : "That chat isn't on this Mac any more."))
            }
        case .cancel:
            // A phone stops only the asks it started.
            if let running = model.running, peer.asked.contains(running.reply) { model.cancel() }
        case .ping:
            peer.session.send(["type": "pong"])
        case .style(let chat, let title, let icon, let color):
            if !model.restyle(chat, title: title, icon: icon, color: color) {
                peer.session.send(PhoneMessage.refused("Couldn't change that chat."))
            }
        case .diagrams:
            peer.session.send(PhoneMessage.diagrams(model.diagrams))
        case nil:
            peer.session.send(PhoneMessage.refused("The Mac didn't understand that request."))
        }
    }

    private func relay(_ update: ChatModel.Update) {
        guard let model else { return }
        for peer in peers.values where peer.session.device != nil {
            switch update {
            case .saved(let chat):
                peer.session.send(PhoneMessage.chats(model.chats, busy: model.running?.chat))
                if peer.open == chat.id, let message = PhoneMessage.chat(chat) { peer.session.send(message) }
            case .live(let chat, let reply, let event):
                peer.session.send(PhoneMessage.live(chat: chat, reply: reply, event: event))
            }
        }
    }

    /// This Mac's addresses on its local networks, for the QR code.
    static func localAddresses() -> [String] {
        var list: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&list) == 0, let first = list else { return [] }
        defer { freeifaddrs(list) }
        var found: [String] = []
        for pointer in sequence(first: first, next: { $0.pointee.ifa_next }) {
            let entry = pointer.pointee
            let flags = Int32(entry.ifa_flags)
            guard let address = entry.ifa_addr, address.pointee.sa_family == UInt8(AF_INET),
                  flags & IFF_UP != 0, flags & IFF_LOOPBACK == 0 else { continue }
            var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            guard getnameinfo(address, socklen_t(address.pointee.sa_len), &host, socklen_t(host.count), nil, 0, NI_NUMERICHOST) == 0 else { continue }
            let text = String(cString: host)
            if LocalNetwork.isLocal(text), !text.hasPrefix("169.254."), !found.contains(text) { found.append(text) }
        }
        return Array(found.prefix(4))
    }

    /// The QR code for `text`, crisp at any size.
    static func qrImage(_ text: String) -> NSImage? {
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(text.utf8)
        // High correction: up to 30% of the code can be covered, so Astrid's mark
        // in the middle (about 5% of it) never stops it scanning.
        filter.correctionLevel = "H"
        guard let output = filter.outputImage?.transformed(by: CGAffineTransform(scaleX: 10, y: 10)),
              let image = CIContext().createCGImage(output, from: output.extent) else { return nil }
        return NSImage(cgImage: image, size: NSSize(width: output.extent.width / 2, height: output.extent.height / 2))
    }
}

/// One phone's WebSocket connection.
private final class Peer {
    let connection: NWConnection
    let session: LinkSession
    /// The chat this phone has open, to send it again when it changes.
    var open: UUID?
    /// Replies to asks this phone started.
    var asked: Set<UUID> = []
    var onEnd: () -> Void = {}
    private var ended = false

    init(connection: NWConnection, session: LinkSession) {
        self.connection = connection
        self.session = session
    }

    func start() {
        session.write = { [weak self] frame in self?.write(frame) }
        session.onClose = { [weak self] closing in
            // A pairing reply is still in flight: close once it is written.
            guard let self else { return }
            if case .paired = closing {
                self.connection.send(content: nil, contentContext: .finalMessage, isComplete: true, completion: .contentProcessed { _ in self.end() })
            } else {
                self.end()
            }
        }
        connection.stateUpdateHandler = { [weak self] state in
            switch state {
            case .failed, .cancelled: self?.end()
            default: break
            }
        }
        connection.start(queue: .main)
        receive()
        DispatchQueue.main.asyncAfter(deadline: .now() + LinkLimits.handshakeTimeout) { [weak self] in
            guard let self, self.session.device == nil, !self.session.isClosed else { return }
            self.session.close(.protocolError("handshake timed out"))
        }
    }

    private func receive() {
        connection.receiveMessage { [weak self] content, context, _, error in
            guard let self, !self.ended else { return }
            if error != nil { return self.end() }
            let metadata = context?.protocolMetadata(definition: NWProtocolWebSocket.definition) as? NWProtocolWebSocket.Metadata
            switch metadata?.opcode {
            case .text:
                self.session.receive(.text(String(decoding: content ?? Data(), as: UTF8.self)))
            case .binary:
                self.session.receive(.binary(content ?? Data()))
            case .close:
                return self.end()
            default:
                break
            }
            if !self.session.isClosed { self.receive() }
        }
    }

    private func write(_ frame: LinkSession.Frame) {
        let metadata: NWProtocolWebSocket.Metadata
        let data: Data
        switch frame {
        case .text(let text):
            metadata = NWProtocolWebSocket.Metadata(opcode: .text)
            data = Data(text.utf8)
        case .binary(let bytes):
            metadata = NWProtocolWebSocket.Metadata(opcode: .binary)
            data = bytes
        }
        let context = NWConnection.ContentContext(identifier: "frame", metadata: [metadata])
        connection.send(content: data, contentContext: context, isComplete: true, completion: .contentProcessed { _ in })
    }

    func end() {
        guard !ended else { return }
        ended = true
        session.close(.protocolError("ended"))
        connection.cancel()
        onEnd()
    }
}
