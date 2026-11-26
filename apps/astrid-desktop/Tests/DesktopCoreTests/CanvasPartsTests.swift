import DesktopCore
import XCTest

final class CanvasPartsTests: XCTestCase {
    private func shape(_ kind: String, id: String? = nil) -> JSONValue {
        var object: [String: JSONValue] = ["shape": .string(kind)]
        if let id { object["id"] = .string(id) }
        return .object(object)
    }

    func testPartIdsAreListedOnceInOrderWithoutEdits() {
        let shapes = [shape("title"), shape("slab", id: "dam"), shape("rotor", id: "turbine"), shape("slab", id: "dam"), shape("edit", id: "ghost")]
        XCTAssertEqual(CanvasParts.ids(in: shapes), ["dam", "turbine"])
        XCTAssertTrue(CanvasParts.isEdit(shape("edit", id: "x")))
        XCTAssertFalse(CanvasParts.isEdit(shape("slab")))
    }

    func testHistoryNamesTheDrawingsPartsSoFollowUpsCanEditThem() {
        var chat = ChatSession(first: ChatMessage(role: .user, text: "draw a dam"))
        let drawing = [JSONValue.object(["shape": .string("title"), "text": .string("Dam")]), shape("rotor", id: "turbine")]
        chat.append(ChatMessage(role: .astrid, text: "Here.", diagram: drawing))
        XCTAssertEqual(chat.history().last?["text"], "Here. [Drew on the canvas: Dam; part ids: turbine]")
        XCTAssertEqual(chat.latestDiagram?.count, 2)
    }

    func testExportsAreCheckedAndNamedSafely() {
        XCTAssertNotNil(CanvasExport.data(format: .svg, payload: "<svg xmlns='http://www.w3.org/2000/svg'/>"))
        XCTAssertNil(CanvasExport.data(format: .svg, payload: "<script>x</script>"))
        let png = Data([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]).base64EncodedString()
        XCTAssertNotNil(CanvasExport.data(format: .png, payload: png))
        XCTAssertNil(CanvasExport.data(format: .png, payload: Data("GIF89a".utf8).base64EncodedString()))
        XCTAssertEqual(CanvasExport.fileName("../../etc/Dam.svg", format: .png), "etc Dam.png")
        XCTAssertEqual(CanvasExport.fileName("", format: .svg), "Astrid drawing.svg")
    }
}
