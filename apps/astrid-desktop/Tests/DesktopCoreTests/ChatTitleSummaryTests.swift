import XCTest
@testable import DesktopCore

final class ChatTitleSummaryTests: XCTestCase {
    func testSummariesAreCleanedToOneLine() {
        XCTAssertEqual(ChatTitle.summary(from: "\"Plan a weekend trip to Lisbon.\"\nExtra"), "Plan a weekend trip to Lisbon")
        XCTAssertEqual(ChatTitle.summary(from: "Title: **heat pump basics**"), "Heat pump basics")
        XCTAssertNil(ChatTitle.summary(from: "  \n```\n"))
        XCTAssertLessThanOrEqual(ChatTitle.summary(from: String(repeating: "word ", count: 40))!.count, ChatTitle.maxSummary)
        XCTAssertTrue(ChatTitle.summaryPrompt(for: "hi").contains("at most 7 words"))
    }

    func testSummaryReplacesTheDerivedTitleButNeverARename() {
        var chat = ChatSession(first: ChatMessage(role: .user, text: "hey can you explain how my heat pump works in winter"))
        XCTAssertTrue(chat.applySummary("How heat pumps work in winter"))
        XCTAssertEqual(chat.title, "How heat pumps work in winter")
        let renamed = ChatStyle.apply(to: chat, title: "Mine", icon: nil, color: nil)!
        XCTAssertEqual(renamed.titleSource, "user")
        var kept = renamed
        XCTAssertFalse(kept.applySummary("Something else"))
        XCTAssertEqual(kept.title, "Mine")
    }

    func testPinnedChatsSortFirst() throws {
        var old = ChatSession(first: ChatMessage(role: .user, text: "old", at: Date(timeIntervalSince1970: 1)))
        let new = ChatSession(first: ChatMessage(role: .user, text: "new", at: Date(timeIntervalSince1970: 100)))
        XCTAssertEqual(ChatStore.sorted([old, new]).first?.id, new.id)
        old.pinned = true
        XCTAssertEqual(ChatStore.sorted([new, old]).first?.id, old.id)
        // Older files without the new fields still decode.
        let data = try JSONEncoder().encode(new)
        var object = try JSONSerialization.jsonObject(with: data) as! [String: Any]
        object["pinned"] = nil
        object["titleSource"] = nil
        XCTAssertNoThrow(try JSONDecoder().decode(ChatSession.self, from: JSONSerialization.data(withJSONObject: object)))
    }
}
