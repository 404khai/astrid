import XCTest
@testable import DesktopCore

final class GroupChatTests: XCTestCase {
    let members = [GroupMember(id: "a#0", name: "Pip"), GroupMember(id: "b#0", name: "Pipkin"), GroupMember(id: "c#0", name: "Hob")]

    func testMentionsMatchWholeNamesInOrder() {
        XCTAssertEqual(Mentions.find(in: "@hob and @Pipkin, ask @astrid", members: members), ["c#0", "b#0", "astrid"])
        XCTAssertEqual(Mentions.find(in: "@Pip!", members: members), ["a#0"])
        XCTAssertEqual(Mentions.find(in: "@Pipx and email@hob.com", members: members), [])
        XCTAssertEqual(Mentions.find(in: "no tags", members: members), [])
    }

    func testUntaggedMessagesGoToAstridAndTagsQueueSpeakers() {
        var plan = GroupTurnPlan()
        plan.start(mentions: [])
        XCTAssertEqual(plan.next(), "astrid")
        plan.replied("astrid", mentions: ["a#0", "astrid", "you"])
        XCTAssertEqual(plan.next(), "a#0")
        XCTAssertNil(plan.next())
    }

    func testRoundsAreBounded() {
        var plan = GroupTurnPlan()
        plan.start(mentions: ["a#0"])
        var turns = 0
        while let speaker = plan.next() {
            turns += 1
            plan.replied(speaker, mentions: [speaker == "a#0" ? "b#0" : "a#0"])
        }
        XCTAssertEqual(turns, 2 * GroupTurnPlan.maxPerSpeaker)
        XCTAssertLessThanOrEqual(turns, GroupTurnPlan.maxTurns)
    }

    func testReplyTrailerIsParsedAndHidden() {
        let reply = GroupReply.parse("Sure, on it @Hob.\n```group\n{\"mood\":\"celebrate\",\"react\":\"🎉\",\"remember\":\"I write the copy\"}\n```")
        XCTAssertEqual(reply.text, "Sure, on it @Hob.")
        XCTAssertEqual(reply.mood, "celebrate")
        XCTAssertEqual(reply.react, "🎉")
        XCTAssertEqual(reply.remember, "I write the copy")
        XCTAssertEqual(GroupReply.parse("plain").text, "plain")
        XCTAssertNil(GroupReply.parse("x ```group {\"mood\":\"rm -rf\"}```").mood)
        XCTAssertEqual(GroupReply.visible("Hi there ```gro"), "Hi there")
    }

    func testReactionsToggleAndAreBounded() {
        var m = GroupMessage(author: .you, text: "hi")
        m.toggle("👍", by: .astrid)
        m.toggle("👍", by: .goblin("a#0"))
        XCTAssertEqual(m.reactions["👍"], ["astrid", "a#0"])
        m.toggle("👍", by: .astrid)
        XCTAssertEqual(m.reactions["👍"], ["a#0"])
        for e in ["1", "2", "3", "4", "5", "6", "7"] { m.toggle(e, by: .you) }
        XCTAssertEqual(m.reactions.count, GroupReactions.maxPerMessage)
    }

    func testThreadsAndMemoryRoundTripPrivately() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let store = GroupThreadStore(directory: dir)
        var thread = GroupThread(group: UUID())
        thread.append(GroupMessage(author: .goblin("a#0"), text: "hello", mood: "happy", mentions: ["astrid"]))
        try store.save(thread)
        XCTAssertEqual(store.load(thread.group), thread)
        let mode = try FileManager.default.attributesOfItem(atPath: dir.appendingPathComponent("\(thread.group.uuidString).json").path)[.posixPermissions] as? Int
        XCTAssertEqual(mode, 0o600)

        let memories = GoblinMemoryStore(directory: dir)
        var memory = GoblinMemory()
        memory.remember("I test things")
        memory.remember("i test things")
        XCTAssertEqual(memory.facts.count, 1)
        try memories.save(memory, for: "07-tansy.goblin.json#0")
        XCTAssertEqual(memories.load("07-tansy.goblin.json#0"), memory)
    }

    func testPromptCarriesPurposeRoleMemoryAndHandoff() {
        var group = GoblinGroup(name: "Launch", members: ["a#0", "c#0"], about: "Ship the website")
        group.setRole("copywriter", for: "a#0")
        var memory = GoblinMemory()
        memory.remember("Prefers short lines")
        let handoff = GroupMessage(author: .goblin("c#0"), text: "@Pip write the hero", mentions: ["a#0"])
        let prompt = GroupPrompt.build(group: group, speaker: "a#0", members: members, thread: [handoff], memory: memory, handoff: handoff)
        for part in ["You are Pip", "Ship the website", "Your role here: copywriter", "Prefers short lines", "Hob tagged you", "```group"] {
            XCTAssertTrue(prompt.contains(part), part)
        }
        XCTAssertTrue(GroupPrompt.build(group: group, speaker: "astrid", members: members, thread: [], memory: GoblinMemory(), handoff: nil).contains("overseer"))
    }
}
