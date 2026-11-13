import XCTest
@testable import DesktopCore

final class ChatTitleTests: XCTestCase {
    func testTitlesComeFromTheFirstSentenceWithoutOpeners() {
        XCTAssertEqual(ChatTitle.derive(from: "hey astrid, can you draw how a heat pump works?"), "Draw how a heat pump works")
        XCTAssertEqual(ChatTitle.derive(from: "Please summarise my inbox. Then list todos."), "Summarise my inbox")
        XCTAssertEqual(ChatTitle.derive(from: "  what's\n\nin   my calendar   "), "What's in my calendar")
        XCTAssertEqual(ChatTitle.derive(from: "Could you please explain TCP"), "Explain TCP")
    }

    func testOpenersOnlyMatchWholeWords() {
        XCTAssertEqual(ChatTitle.derive(from: "hiking trails near me"), "Hiking trails near me")
        XCTAssertEqual(ChatTitle.derive(from: "Sorting algorithms"), "Sorting algorithms")
    }

    func testLongTitlesAreCutAtWordsWithAnEllipsis() {
        XCTAssertEqual(ChatTitle.derive(from: "draw the full anatomy of a modern smartphone with every part labelled"), "Draw the full anatomy of a…")
        let long = ChatTitle.derive(from: "Supercalifragilisticexpialidociousnessessessess everywhere")
        XCTAssertLessThanOrEqual(long.count, 41)
        XCTAssertTrue(long.hasSuffix("…"))
    }

    func testEmptyOrOnlyOpenersFallBack() {
        XCTAssertEqual(ChatTitle.derive(from: ""), ChatTitle.fallback)
        XCTAssertEqual(ChatTitle.derive(from: "   \n "), ChatTitle.fallback)
        XCTAssertEqual(ChatTitle.derive(from: "Hey Astrid!"), ChatTitle.fallback)
        XCTAssertEqual(ChatTitle.derive(from: "?!"), ChatTitle.fallback)
    }

    func testTitlesAreDeterministic() {
        let text = "So, can you sketch a bicycle drivetrain for me?"
        XCTAssertEqual(ChatTitle.derive(from: text), ChatTitle.derive(from: text))
        XCTAssertEqual(ChatTitle.derive(from: text), "Sketch a bicycle drivetrain for me")
    }
}

final class ChatStoreTests: XCTestCase {
    private var directory: URL!

    override func setUp() {
        directory = FileManager.default.temporaryDirectory.appendingPathComponent("astrid-chats-\(UUID().uuidString)")
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: directory)
    }

    private let shapes: [JSONValue] = [
        .object(["shape": .string("title"), "text": .string("Heat pump")]),
        .object(["shape": .string("slab"), "x": .number(-40), "y": .number(0), "w": .number(80), "d": .number(40), "h": .number(30), "dashed": .bool(true)]),
        .object(["shape": .string("line"), "points": .array([.array([.number(0), .number(0)]), .array([.number(10), .number(20)])])]),
    ]

    func testChatsRoundTripWithSentMessagesRepliesAndDiagrams() throws {
        let store = ChatStore(directory: directory)
        var chat = ChatSession(first: ChatMessage(role: .user, text: "draw a heat pump", at: Date(timeIntervalSince1970: 100)))
        chat.append(ChatMessage(role: .astrid, text: "Here's a heat pump.", diagram: shapes, outcome: "completed", at: Date(timeIntervalSince1970: 130)))
        try store.save(chat)

        let loaded = store.loadAll()
        XCTAssertEqual(loaded, [chat])
        XCTAssertEqual(loaded[0].title, "Draw a heat pump")
        XCTAssertEqual(loaded[0].messages.map(\.role), [.user, .astrid])
        XCTAssertEqual(loaded[0].messages[1].diagram, shapes)
        XCTAssertEqual(loaded[0].messages[1].diagramTitle, "Heat pump")
        XCTAssertEqual(loaded[0].updated, Date(timeIntervalSince1970: 130))

        let file = directory.appendingPathComponent("\(chat.id.uuidString.lowercased()).json")
        let filePermissions = try FileManager.default.attributesOfItem(atPath: file.path)[.posixPermissions] as? Int
        let folderPermissions = try FileManager.default.attributesOfItem(atPath: directory.path)[.posixPermissions] as? Int
        XCTAssertEqual(filePermissions, 0o600)
        XCTAssertEqual(folderPermissions, 0o700)
    }

    func testChatsAreListedByMostRecentActivity() throws {
        let store = ChatStore(directory: directory)
        let old = ChatSession(first: ChatMessage(role: .user, text: "old", at: Date(timeIntervalSince1970: 10)))
        var revived = ChatSession(first: ChatMessage(role: .user, text: "revived", at: Date(timeIntervalSince1970: 20)))
        let recent = ChatSession(first: ChatMessage(role: .user, text: "recent", at: Date(timeIntervalSince1970: 30)))
        revived.append(ChatMessage(role: .user, text: "again", at: Date(timeIntervalSince1970: 40)))
        for chat in [old, revived, recent] { try store.save(chat) }
        XCTAssertEqual(store.loadAll().map(\.title), ["Revived", "Recent", "Old"])
    }

    func testUnreadableNewerAndMisnamedFilesAreSkippedAndKept() throws {
        let store = ChatStore(directory: directory)
        let good = ChatSession(first: ChatMessage(role: .user, text: "good"))
        try store.save(good)
        var future = ChatSession(first: ChatMessage(role: .user, text: "future"))
        future.v = ChatSession.currentVersion + 1
        try store.save(future)
        let garbage = directory.appendingPathComponent("\(UUID().uuidString.lowercased()).json")
        try Data("{not json".utf8).write(to: garbage)
        let misnamed = directory.appendingPathComponent("\(UUID().uuidString.lowercased()).json")
        try FileManager.default.copyItem(at: directory.appendingPathComponent("\(good.id.uuidString.lowercased()).json"), to: misnamed)

        XCTAssertEqual(store.loadAll().map(\.id), [good.id])
        XCTAssertTrue(FileManager.default.fileExists(atPath: garbage.path))
    }

    func testAMissingFolderIsAnEmptyList() {
        XCTAssertEqual(ChatStore(directory: directory).loadAll(), [])
    }

    func testMessagesAreBounded() {
        let message = ChatMessage(role: .astrid, text: String(repeating: "x", count: ChatLimits.maxText + 10), diagram: Array(repeating: .null, count: 300))
        XCTAssertEqual(message.text.count, ChatLimits.maxText)
        XCTAssertEqual(message.diagram?.count, ChatLimits.maxShapes)
    }

    func testHistoryDescribesRepliesAndDrawingsForFollowUps() {
        var chat = ChatSession(first: ChatMessage(role: .user, text: "draw a heat pump"))
        chat.append(ChatMessage(role: .astrid, text: "Here it is.", diagram: shapes))
        chat.append(ChatMessage(role: .astrid, text: "", brief: .object(["title": .string("Inbox"), "summary": .string("Two replies due")])))
        chat.append(ChatMessage(role: .astrid, text: "  ", outcome: "cancelled"))
        XCTAssertEqual(chat.history(), [
            ["role": "user", "text": "draw a heat pump"],
            ["role": "astrid", "text": "Here it is. [Drew on the canvas: Heat pump]"],
            ["role": "astrid", "text": "Inbox. Two replies due"],
        ])
    }
}

final class AskEventTests: XCTestCase {
    private func parse(_ line: String) -> AskEvent? { AskEvent.parse(Data(line.utf8)) }

    func testRuntimeLinesParse() {
        XCTAssertEqual(parse(#"{"type":"ready","model":"m"}"#), .ready)
        XCTAssertEqual(parse(#"{"type":"state","state":"working","label":"Thinking"}"#), .state(state: "working", label: "Thinking"))
        XCTAssertEqual(parse(#"{"type":"delta","text":"Hi"}"#), .delta("Hi"))
        XCTAssertEqual(parse(#"{"type":"approval","id":3,"title":"Search Gmail?","detail":{}}"#), .approval(id: 3, title: "Search Gmail?"))
        XCTAssertEqual(parse(#"{"type":"answer","text":"Done","brief":{"title":"T","done":true},"canvas":true}"#),
                       .answer(text: "Done", brief: .object(["title": .string("T"), "done": .bool(true)]), drew: true))
        XCTAssertEqual(parse(#"{"type":"answer","text":"Plain","brief":null}"#), .answer(text: "Plain", brief: nil, drew: false))
        XCTAssertEqual(parse(#"{"type":"canvas","event":"begin","width":1000,"height":640}"#), .canvasBegin)
        XCTAssertEqual(parse(#"{"type":"canvas","event":"shape","shape":{"shape":"rect","x":1,"w":2}}"#),
                       .canvasShape(.object(["shape": .string("rect"), "x": .number(1), "w": .number(2)])))
        XCTAssertEqual(parse(#"{"type":"canvas","event":"end"}"#), .canvasEnd)
        XCTAssertEqual(parse(#"{"type":"canvas","event":"cancel"}"#), .canvasCancel)
        XCTAssertEqual(parse(#"{"type":"error","message":"no"}"#), .error("no"))
        XCTAssertEqual(parse(#"{"type":"done","outcome":"completed"}"#), .done(outcome: "completed"))
    }

    func testMalformedLinesAreIgnored() {
        XCTAssertNil(parse("not json"))
        XCTAssertNil(parse(#"["type"]"#))
        XCTAssertNil(parse(#"{"type":"unknown"}"#))
        XCTAssertNil(parse(#"{"type":"canvas","event":"shape","shape":"rect"}"#))
        XCTAssertNil(parse(#"{"type":"approval","id":"x"}"#))
    }

    func testLinesSplitAcrossChunksAndOverlongLinesAreDropped() {
        var buffer = LineBuffer(limit: 16)
        XCTAssertEqual(buffer.append(Data("{\"a\":".utf8)), [])
        XCTAssertEqual(buffer.append(Data("1}\n{\"b\":2}\n{".utf8)), [Data("{\"a\":1}".utf8), Data("{\"b\":2}".utf8)])
        XCTAssertEqual(buffer.append(Data(String(repeating: "x", count: 32).utf8)), [])
        XCTAssertEqual(buffer.append(Data("{}\n".utf8)), [Data("{}".utf8)])
    }

    func testTheChildIsAReadOnlyPresenceAskWithOptionalMail() {
        let url = URL(fileURLWithPath: "/usr/bin/false")
        XCTAssertEqual(AskChild(config: .init(astrid: url, model: "claude-code/sonnet", mail: "gmail")).arguments,
                       ["presence-ask", "--model", "claude-code/sonnet", "--mail-source", "gmail"])
        XCTAssertEqual(AskChild(config: .init(astrid: url, model: "m", mail: nil)).arguments, ["presence-ask", "--model", "m"])
    }
}
