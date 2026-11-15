import XCTest
@testable import PresenceCore

final class SpokenTextTests: XCTestCase {
    func testDateAbbreviationsAreSpokenInFull() {
        XCTAssertEqual(SpokenText.expand("Can I move your 3pm Design review to 3pm on Wednesday, Nov 25?"),
                       "Can I move your 3 PM Design review to 3 PM on Wednesday, November 25?")
        XCTAssertEqual(SpokenText.expand("Mon, Nov 16 · 2:00 PM – 5:00 PM"), "Monday, November 16 · 2:00 PM – 5:00 PM")
        XCTAssertEqual(SpokenText.expand("Due Fri 20 Nov."), "Due Friday 20 November")
        XCTAssertEqual(SpokenText.expand("Sept. 3 at 4:30pm"), "September 3 at 4:30 PM")
        XCTAssertEqual(SpokenText.expand("Tue Dec 1"), "Tuesday December 1")
    }

    func testOrdinaryWordsAreLeftAlone() {
        for text in ["May I add it?", "The Sun is out.", "Mar the finish", "Wed to the idea", "Sat down at the desk", "Decide on Jan"] {
            XCTAssertEqual(SpokenText.expand(text), text)
        }
    }
}
