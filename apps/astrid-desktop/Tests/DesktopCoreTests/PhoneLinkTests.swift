import CryptoKit
import XCTest
@testable import DesktopCore

/// The phone's side of a connection, as `mobile/src/link` does it.
private struct FakePhone {
    var cipher: LinkCipher?
    let nonce = LinkCrypto.random(LinkCrypto.nonceBytes)

    func hello(device: String?) -> LinkSession.Frame {
        var object: [String: Any] = ["t": "hello", "v": 1, "nonce": nonce.base64EncodedString()]
        object["device"] = device ?? NSNull()
        return .text(LinkSession.json(object))
    }

    mutating func answer(_ frame: LinkSession.Frame, secret: Data, label: String) {
        guard case .text(let text) = frame,
              let object = try? JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any],
              let mac = (object["nonce"] as? String).flatMap({ Data(base64Encoded: $0) }) else { return XCTFail("no hello") }
        cipher = LinkCipher(key: LinkCrypto.sessionKey(secret: secret, phoneNonce: nonce, macNonce: mac, label: label), sends: .phone)
    }

    mutating func seal(_ message: [String: Any]) -> LinkSession.Frame {
        .binary(try! cipher!.seal(JSONSerialization.data(withJSONObject: message)))
    }

    mutating func open(_ frame: LinkSession.Frame) -> [String: Any]? {
        guard case .binary(let data) = frame, let plaintext = try? cipher!.open(data) else { return nil }
        return try? JSONSerialization.jsonObject(with: plaintext) as? [String: Any]
    }
}

private final class Harness {
    let registry: DeviceRegistry
    var offer: PairingOffer?
    var written: [LinkSession.Frame] = []
    var messages: [[String: Any]] = []
    var closed: LinkSession.Closing?
    lazy var session: LinkSession = {
        let session = LinkSession(registry: registry, macName: "Test Mac") { [unowned self] peek in
            guard let offer = self.offer, offer.isValid() else { return nil }
            if !peek { self.offer = nil }
            return offer.secret
        }
        session.write = { [unowned self] in self.written.append($0) }
        session.onMessage = { [unowned self] in self.messages.append($0) }
        session.onClose = { [unowned self] in self.closed = $0 }
        return session
    }()

    init(directory: URL) {
        registry = DeviceRegistry(file: directory.appendingPathComponent("Devices.json"))
    }
}

final class PhoneLinkTests: XCTestCase {
    private var directory: URL!

    override func setUpWithError() throws {
        directory = FileManager.default.temporaryDirectory.appendingPathComponent("astrid-link-\(UUID().uuidString)")
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: directory)
    }

    /// The same numbers are checked by `mobile/src/link/crypto.test.mjs`, so
    /// the Mac and the phone agree on the key, the nonce and the sealing.
    func testFixedVectorMatchesThePhone() throws {
        let secret = Data((0..<32).map { UInt8($0) })
        let phone = Data((0..<32).map { UInt8(0x40 + $0) })
        let mac = Data((0..<32).map { UInt8(0x80 + $0) })
        let key = LinkCrypto.sessionKey(secret: secret, phoneNonce: phone, macNonce: mac, label: "pair")
        let keyHex = key.withUnsafeBytes { Data($0) }.map { String(format: "%02x", $0) }.joined()
        XCTAssertEqual(keyHex, "913dde7addb42a6c328279b708c3b1293ba3eeecb2cb78b60c76e8bcde8cc6eb")
        var cipher = LinkCipher(key: key, sends: .mac)
        let first = try cipher.seal(Data(#"{"type":"pong"}"#.utf8))
        let second = try cipher.seal(Data(#"{"type":"pong"}"#.utf8))
        XCTAssertEqual(first.map { String(format: "%02x", $0) }.joined(), "5ddd20fb7dbeb0d45560bb529576d0fdf2b085a740514d48855b2af09c57fb")
        XCTAssertNotEqual(first, second)
    }

    func testCipherRejectsReplayReorderAndTampering() throws {
        let key = SymmetricKey(size: .bits256)
        var mac = LinkCipher(key: key, sends: .mac)
        var phone = LinkCipher(key: key, sends: .phone)
        let one = try mac.seal(Data("one".utf8))
        let two = try mac.seal(Data("two".utf8))
        XCTAssertThrowsError(try phone.open(two), "out of order")
        XCTAssertEqual(try phone.open(one), Data("one".utf8))
        XCTAssertThrowsError(try phone.open(one), "replayed")
        var tampered = two
        tampered[0] ^= 1
        XCTAssertThrowsError(try phone.open(tampered))
        XCTAssertEqual(try phone.open(two), Data("two".utf8))
        // A frame the phone sealed can't be reflected back to it as the Mac's.
        let fromPhone = try phone.seal(Data("hi".utf8))
        var otherPhone = LinkCipher(key: key, sends: .phone)
        XCTAssertThrowsError(try otherPhone.open(fromPhone))
    }

    func testPairingUsesTheCodeOnceAndHandsOverAKey() throws {
        let harness = Harness(directory: directory)
        let offer = PairingOffer()
        harness.offer = offer
        var phone = FakePhone()
        harness.session.receive(phone.hello(device: nil))
        XCTAssertEqual(harness.written.count, 1)
        phone.answer(harness.written[0], secret: offer.secret, label: "pair")
        harness.session.receive(phone.seal(["type": "pair", "name": "Pixel\n8"]))
        guard case .paired(let device) = harness.closed else { return XCTFail("not paired: \(String(describing: harness.closed))") }
        XCTAssertNil(harness.offer, "code used up")
        XCTAssertEqual(device.name, "Pixel8")
        let reply = phone.open(harness.written[1])
        XCTAssertEqual(reply?["type"] as? String, "paired")
        XCTAssertEqual(reply?["device"] as? String, device.id)
        XCTAssertEqual((reply?["key"] as? String).flatMap { Data(base64Encoded: $0) }, device.key)
        XCTAssertEqual(reply?["name"] as? String, "Test Mac")

        // Saved privately, and found again on the next launch.
        let attributes = try FileManager.default.attributesOfItem(atPath: harness.registry.file.path)
        XCTAssertEqual(attributes[.posixPermissions] as? Int, 0o600)
        let reloaded = DeviceRegistry(file: harness.registry.file).device(device.id)
        XCTAssertEqual(reloaded?.key, device.key)
        XCTAssertEqual(reloaded?.name, device.name)

        // A second phone with the same code gets nothing.
        let again = Harness(directory: directory)
        again.offer = nil
        again.session.receive(FakePhone().hello(device: nil))
        XCTAssertEqual(again.closed, .notPairing)
        XCTAssertTrue(again.written.isEmpty)
    }

    func testPairingNeedsTheRightSecretAndALiveCode() {
        let expired = Harness(directory: directory)
        expired.offer = PairingOffer(now: Date().addingTimeInterval(-LinkLimits.pairingLifetime - 1))
        expired.session.receive(FakePhone().hello(device: nil))
        XCTAssertEqual(expired.closed, .notPairing)

        let wrong = Harness(directory: directory)
        wrong.offer = PairingOffer()
        var phone = FakePhone()
        wrong.session.receive(phone.hello(device: nil))
        phone.answer(wrong.written[0], secret: LinkCrypto.random(32), label: "pair")
        wrong.session.receive(phone.seal(["type": "pair", "name": "Eve"]))
        XCTAssertEqual(wrong.closed, .unauthentic)
        XCTAssertNotNil(wrong.offer, "a failed guess doesn't use up the code")
        XCTAssertTrue(wrong.registry.devices.isEmpty)
    }

    func testPairedPhoneOpensTheLinkAndRevokedOneCannot() throws {
        let harness = Harness(directory: directory)
        let device = try harness.registry.add(name: "Pixel")
        var phone = FakePhone()
        harness.session.receive(phone.hello(device: device.id))
        phone.answer(harness.written[0], secret: device.key, label: device.id)
        harness.session.receive(phone.seal(["type": "sync"]))
        XCTAssertNil(harness.closed)
        XCTAssertEqual(harness.session.device?.id, device.id)
        XCTAssertEqual(harness.messages.first?["type"] as? String, "sync")
        XCTAssertTrue(harness.session.send(["type": "pong"]))
        XCTAssertEqual(phone.open(harness.written[1])?["type"] as? String, "pong")
        XCTAssertNotNil(harness.registry.device(device.id)?.lastSeen)

        // A text frame after the hello ends it.
        harness.session.receive(.text("{}"))
        XCTAssertEqual(harness.closed, .protocolError("unexpected frame"))
        XCTAssertFalse(harness.session.send(["type": "pong"]))

        try harness.registry.remove(device.id)
        let revoked = Harness(directory: directory)
        revoked.session.receive(FakePhone().hello(device: device.id))
        XCTAssertEqual(revoked.closed, .unknownDevice)
    }

    func testMalformedHellosAndOversizeFramesClose() throws {
        for hello in [#"{"t":"hello","v":2,"nonce":"AA=="}"#, "nope", #"{"t":"hello","v":1,"nonce":"AAAA"}"#] {
            let harness = Harness(directory: directory)
            harness.session.receive(.text(hello))
            XCTAssertEqual(harness.closed, .protocolError("bad hello"))
        }
        let harness = Harness(directory: directory)
        let device = try harness.registry.add(name: "Pixel")
        var phone = FakePhone()
        harness.session.receive(phone.hello(device: device.id))
        phone.answer(harness.written[0], secret: device.key, label: device.id)
        harness.session.receive(.binary(Data(count: LinkLimits.maxInbound + 100)))
        XCTAssertEqual(harness.closed, .protocolError("frame too large"))
    }

    func testLocalNetworkOnly() {
        for host in ["127.0.0.1", "10.0.2.2", "192.168.1.20", "172.20.0.4", "169.254.3.3", "::1", "fe80::1%en0", "fd12::3", "::ffff:192.168.0.2"] {
            XCTAssertTrue(LocalNetwork.isLocal(host), host)
        }
        for host in ["8.8.8.8", "172.32.0.1", "100.64.0.1", "2001:db8::1", "example.com", "192.168.1"] {
            XCTAssertFalse(LocalNetwork.isLocal(host), host)
        }
    }

    func testPairingURLCarriesWhereAndTheSecret() throws {
        let offer = PairingOffer(secret: Data(repeating: 0xfb, count: 32))
        let url = try XCTUnwrap(URLComponents(string: offer.url(hosts: ["192.168.1.4", "10.0.0.3"], port: 47821, name: "Khai's Mac")))
        XCTAssertEqual(url.scheme, "astrid")
        XCTAssertEqual(url.host, "pair")
        let items = Dictionary(uniqueKeysWithValues: (url.queryItems ?? []).map { ($0.name, $0.value ?? "") })
        XCTAssertEqual(items["h"], "192.168.1.4,10.0.0.3")
        XCTAssertEqual(items["p"], "47821")
        XCTAssertEqual(items["n"], "Khai's Mac")
        XCTAssertEqual(Data(base64URL: items["s"] ?? ""), offer.secret)
        XCTAssertFalse(items["s"]!.contains("+") || items["s"]!.contains("/") || items["s"]!.contains("="))
    }

    func testRegistryKeepsAtMostEightPhones() throws {
        let registry = DeviceRegistry(file: directory.appendingPathComponent("Devices.json"))
        var first: PairedDevice?
        for index in 0..<10 {
            let device = try registry.add(name: "Phone \(index)")
            if index == 0 { first = device }
        }
        XCTAssertEqual(registry.devices.count, LinkLimits.maxDevices)
        XCTAssertNil(registry.device(first!.id))
    }

    func testPhoneRequests() {
        let id = UUID()
        XCTAssertEqual(PhoneRequest.parse(["type": "sync"]), .sync)
        XCTAssertEqual(PhoneRequest.parse(["type": "open", "chat": id.uuidString]), .open(id))
        XCTAssertNil(PhoneRequest.parse(["type": "open", "chat": "nope"]))
        XCTAssertEqual(PhoneRequest.parse(["type": "ask", "text": "  draw a heart \n"]), .ask(chat: nil, text: "draw a heart"))
        XCTAssertEqual(PhoneRequest.parse(["type": "ask", "chat": id.uuidString, "text": "more"]), .ask(chat: id, text: "more"))
        XCTAssertNil(PhoneRequest.parse(["type": "ask", "chat": "bad", "text": "x"]))
        XCTAssertNil(PhoneRequest.parse(["type": "ask", "text": "   "]))
        if case .ask(_, let text) = PhoneRequest.parse(["type": "ask", "text": String(repeating: "a", count: 9000)]) {
            XCTAssertEqual(text.count, LinkLimits.maxAsk)
        } else { XCTFail() }
        XCTAssertNil(PhoneRequest.parse(["type": "approve", "id": 1]), "phones can't answer approvals")
        XCTAssertNil(PhoneRequest.parse(["type": "shell"]))
    }

    func testChatListAndLiveEvents() throws {
        var chat = ChatSession(first: ChatMessage(role: .user, text: "Draw a heart"))
        chat.append(ChatMessage(role: .astrid, text: "", diagram: [.object(["shape": .string("title"), "text": .string("Heart")])]))
        let list = PhoneMessage.chats([chat], busy: chat.id)
        let rows = try XCTUnwrap(list["chats"] as? [[String: Any]])
        XCTAssertEqual(rows.first?["title"] as? String, "Draw a heart")
        XCTAssertEqual(rows.first?["preview"] as? String, "Drew: Heart")
        XCTAssertEqual(rows.first?["drawings"] as? Int, 1)
        XCTAssertEqual(list["busy"] as? String, chat.id.uuidString)
        let full = try XCTUnwrap(PhoneMessage.chat(chat)?["chat"] as? [String: Any])
        XCTAssertEqual((full["messages"] as? [Any])?.count, 2)
        XCTAssertNoThrow(try JSONSerialization.data(withJSONObject: PhoneMessage.live(chat: chat.id, reply: UUID(), event: .canvasShape(.object(["shape": .string("rect")])))))
        let event = AskEvent.answer(text: "Here", brief: .object(["title": .string("T")]), drew: true).json
        XCTAssertEqual(AskEvent.parse(try JSONSerialization.data(withJSONObject: event)), .answer(text: "Here", brief: .object(["title": .string("T")]), drew: true))
    }
}
