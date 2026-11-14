import DesktopCore
import XCTest

final class ChatStyleTests: XCTestCase {
    private let chat = ChatSession(first: ChatMessage(role: .user, text: "Draw the brain"))

    func testRenameIconAndColourAreValidatedAndResettable() throws {
        let renamed = try XCTUnwrap(ChatStyle.apply(to: chat, title: "  Brain\nparts  ", icon: nil, color: nil))
        XCTAssertEqual(renamed.title, "Brain parts")
        XCTAssertNil(ChatStyle.apply(to: chat, title: "   ", icon: nil, color: nil), "an empty name is refused")
        XCTAssertNil(ChatStyle.apply(to: chat, title: nil, icon: "skull", color: nil), "unknown icons are refused")
        XCTAssertNil(ChatStyle.apply(to: chat, title: nil, icon: nil, color: "#ff0000"), "only named colours")
        let styled = try XCTUnwrap(ChatStyle.apply(to: chat, title: nil, icon: "brain", color: "pink"))
        XCTAssertEqual(styled.icon, "brain")
        XCTAssertEqual(ChatStyle.hex(styled.color), "#E5487F")
        let reset = try XCTUnwrap(ChatStyle.apply(to: styled, title: nil, icon: "", color: ""))
        XCTAssertNil(reset.icon)
        XCTAssertNil(reset.color)
        XCTAssertNil(ChatStyle.apply(to: chat, title: chat.title, icon: nil, color: nil), "no change, no save")
        XCTAssertEqual(ChatStyle.apply(to: chat, title: String(repeating: "x", count: 200), icon: nil, color: nil)?.title.count, ChatStyle.maxTitle)
    }

    func testOldChatFilesStillLoadAndStyleRoundTrips() throws {
        var styled = chat
        styled.icon = "star"
        styled.color = "teal"
        let back = try ChatStore.decode(ChatStore.encode(styled))
        XCTAssertEqual(back.icon, "star")
        XCTAssertEqual(back.color, "teal")
        var raw = try JSONSerialization.jsonObject(with: ChatStore.encode(chat)) as! [String: Any]
        raw["icon"] = nil
        raw["color"] = nil
        XCTAssertNil(try ChatStore.decode(JSONSerialization.data(withJSONObject: raw)).icon)
    }

    func testPhoneStyleAndDiagramsRequests() throws {
        let id = UUID()
        XCTAssertEqual(PhoneRequest.parse(["type": "style", "chat": id.uuidString, "icon": "brain"]), .style(chat: id, title: nil, icon: "brain", color: nil))
        XCTAssertNil(PhoneRequest.parse(["type": "style", "chat": id.uuidString]), "a style request must change something")
        XCTAssertNil(PhoneRequest.parse(["type": "style", "chat": "nope", "title": "x"]))
        XCTAssertEqual(PhoneRequest.parse(["type": "diagrams"]), .diagrams)
        let rows = PhoneMessage.chats([chat], busy: nil)["chats"] as! [[String: Any]]
        XCTAssertEqual(rows[0]["icon"] as? String, "")
    }
}
