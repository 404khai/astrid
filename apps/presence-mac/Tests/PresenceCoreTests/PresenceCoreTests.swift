import CoreGraphics
import ImageIO
import XCTest
@testable import PresenceCore

final class ProtocolTests: XCTestCase {
    func testParsesEveryEventType() throws {
        XCTAssertEqual(try PresenceProtocol.parse(line: #"{"v":1,"type":"show"}"#), .show(at: nil))
        XCTAssertEqual(
            try PresenceProtocol.parse(line: #"{"v":1,"type":"show","x":10,"y":20,"display":1}"#),
            .show(at: PresencePoint(x: 10, y: 20, display: 1)))
        XCTAssertEqual(try PresenceProtocol.parse(line: #"{"v":1,"type":"hide"}"#), .hide)
        XCTAssertEqual(
            try PresenceProtocol.parse(line: #"{"v":1,"type":"point_at","x":12.5,"y":-40}"#),
            .pointAt(PresencePoint(x: 12.5, y: -40)))
        XCTAssertEqual(
            try PresenceProtocol.parse(line: #"{"v":1,"type":"say","text":"Hi"}"#),
            .say(text: "Hi", final: true, durationMs: nil))
        XCTAssertEqual(
            try PresenceProtocol.parse(line: #"{"v":1,"type":"say","text":"Hi","final":false,"duration_ms":900}"#),
            .say(text: "Hi", final: false, durationMs: 900))
        XCTAssertEqual(
            try PresenceProtocol.parse(line: #"{"v":1,"type":"sketch","display":0,"shapes":[{"kind":"circle","x":1,"y":2,"r":3},{"kind":"arrow","from":[0,0],"to":[5,6]},{"kind":"line","from":[1,1],"to":[2,2]}]}"#),
            .sketch(shapes: [
                .circle(center: CGPoint(x: 1, y: 2), radius: 3),
                .arrow(from: .zero, to: CGPoint(x: 5, y: 6)),
                .line(from: CGPoint(x: 1, y: 1), to: CGPoint(x: 2, y: 2)),
            ], display: 0))
        XCTAssertEqual(try PresenceProtocol.parse(line: #"{"v":1,"type":"clear_ink"}"#), .clearInk)
        XCTAssertEqual(try PresenceProtocol.parse(line: #"{"v":1,"type":"wait","ms":250}"#), .wait(ms: 250))
        XCTAssertEqual(try PresenceProtocol.parse(line: #"{"v":1,"type":"quit"}"#), .quit)
    }

    func testIgnoresUnknownFieldsForAdditiveGrowth() throws {
        XCTAssertEqual(try PresenceProtocol.parse(line: #"{"v":1,"type":"hide","future":{"a":1}}"#), .hide)
    }

    func testRejectsMalformedInput() {
        let rejected = [
            "not json",
            "[1,2]",
            #"{"type":"hide"}"#,
            #"{"v":2,"type":"hide"}"#,
            #"{"v":"1","type":"hide"}"#,
            #"{"v":1}"#,
            #"{"v":1,"type":"teleport"}"#,
            #"{"v":1,"type":"point_at","x":1}"#,
            #"{"v":1,"type":"point_at","x":"1","y":2}"#,
            #"{"v":1,"type":"point_at","x":true,"y":2}"#,
            #"{"v":1,"type":"point_at","x":1e9,"y":2}"#,
            #"{"v":1,"type":"point_at","x":1,"y":2,"display":16}"#,
            #"{"v":1,"type":"point_at","x":1,"y":2,"display":1.5}"#,
            #"{"v":1,"type":"say","text":"x","final":1}"#,
            #"{"v":1,"type":"say","text":"x","duration_ms":60001}"#,
            #"{"v":1,"type":"wait"}"#,
            #"{"v":1,"type":"wait","ms":-1}"#,
            #"{"v":1,"type":"sketch","shapes":[]}"#,
            #"{"v":1,"type":"sketch","shapes":[{"kind":"star","x":1,"y":1}]}"#,
            #"{"v":1,"type":"sketch","shapes":[{"kind":"circle","x":1,"y":1,"r":0}]}"#,
            #"{"v":1,"type":"sketch","shapes":[{"kind":"arrow","from":[1],"to":[2,2]}]}"#,
        ]
        for line in rejected {
            XCTAssertThrowsError(try PresenceProtocol.parse(line: line), line)
        }
    }

    func testEnforcesSizeLimits() {
        let longText = String(repeating: "a", count: PresenceProtocol.Limits.maxTextCharacters + 1)
        XCTAssertThrowsError(try PresenceProtocol.parse(line: #"{"v":1,"type":"say","text":"\#(longText)"}"#))
        let shape = #"{"kind":"circle","x":1,"y":1,"r":1}"#
        let many = Array(repeating: shape, count: PresenceProtocol.Limits.maxShapesPerSketch + 1).joined(separator: ",")
        XCTAssertThrowsError(try PresenceProtocol.parse(line: #"{"v":1,"type":"sketch","shapes":[\#(many)]}"#))
        let padded = #"{"v":1,"type":"hide","pad":""# + String(repeating: " ", count: PresenceProtocol.Limits.maxLineBytes) + #""}"#
        XCTAssertThrowsError(try PresenceProtocol.parse(line: padded))
    }

    func testEncodeRoundTrips() throws {
        let events: [PresenceEvent] = [
            .show(at: nil), .show(at: PresencePoint(x: 3, y: 4, display: 1)), .hide,
            .pointAt(PresencePoint(x: 10.5, y: 20)),
            .say(text: "quote \" and é", final: false, durationMs: 1200), .say(text: "", final: true, durationMs: nil),
            .sketch(shapes: [.circle(center: CGPoint(x: 1, y: 2), radius: 9), .arrow(from: .zero, to: CGPoint(x: 1, y: 1))], display: 0),
            .clearInk, .wait(ms: 0), .quit,
        ]
        for event in events {
            let line = PresenceProtocol.encode(event)
            XCTAssertFalse(line.contains("\n"))
            XCTAssertEqual(try PresenceProtocol.parse(line: line), event, line)
        }
    }

    func testScriptReportsLineNumbersAndSkipsComments() {
        let script = PresenceProtocol.parseScript("""
        # tour
        {"v":1,"type":"show"}

        {"v":1,"type":"oops"}
        {"v":1,"type":"hide"}
        """)
        XCTAssertEqual(script.events.map(\.line), [2, 5])
        XCTAssertEqual(script.events.map(\.event), [.show(at: nil), .hide])
        XCTAssertEqual(script.errors.map(\.line), [4])
    }
}

final class PlayerTests: XCTestCase {
    func testWaitsHoldLaterEventsWithoutDrift() {
        var player = ScriptPlayer()
        player.enqueue(contentsOf: [.show(at: nil), .wait(ms: 500), .hide, .wait(ms: 500), .clearInk])
        XCTAssertEqual(player.drain(now: 10), [.show(at: nil)])
        XCTAssertEqual(player.drain(now: 10.49), [])
        // A late tick does not push the next deadline back.
        XCTAssertEqual(player.drain(now: 10.52), [.hide])
        XCTAssertEqual(player.drain(now: 10.99), [])
        XCTAssertEqual(player.drain(now: 11.0), [.clearInk])
        XCTAssertTrue(player.isIdle)
    }

    func testElapsedWaitsCollapseIntoOneDrain() {
        var player = ScriptPlayer()
        player.enqueue(contentsOf: [.wait(ms: 0), .hide, .wait(ms: 100), .quit])
        XCTAssertEqual(player.drain(now: 0), [.hide])
        XCTAssertFalse(player.isIdle)
        XCTAssertEqual(player.drain(now: 5), [.quit])
        player.enqueue(.clearInk)
        XCTAssertEqual(player.drain(now: 5), [.clearInk])
    }

    func testLineAccumulatorBoundsLongLines() {
        var lines = LineAccumulator(maxBytes: 4)
        XCTAssertEqual(lines.feed(Array("ab\ncd".utf8)), [.line("ab")])
        XCTAssertEqual(lines.feed(Array("efgh\nij\n".utf8)), [.overflow, .line("ij")])
        XCTAssertEqual(lines.feed(Array("xy".utf8)), [])
        XCTAssertEqual(lines.finish(), [.line("xy")])
        XCTAssertEqual(lines.finish(), [])
    }
}

final class GeometryTests: XCTestCase {
    // Primary 1512×982 at the origin; a 1920×1080 display to its right whose
    // bottom edge is 100 points lower (Cocoa y = -100).
    let layout = DisplayLayout.fromCocoa(frames: [
        CGRect(x: 0, y: 0, width: 1512, height: 982),
        CGRect(x: 1512, y: -100, width: 1920, height: 1080),
    ])

    func testCocoaFramesBecomeTopLeftGlobalPoints() {
        XCTAssertEqual(layout.displays[0].frame, CGRect(x: 0, y: 0, width: 1512, height: 982))
        XCTAssertEqual(layout.displays[1].frame, CGRect(x: 1512, y: 2, width: 1920, height: 1080))
        XCTAssertEqual(DisplayLayout.cocoaToQuartz(point: CGPoint(x: 5, y: 982), primaryHeight: 982), CGPoint(x: 5, y: 0))
    }

    func testDisplayRelativePointsResolve() throws {
        XCTAssertEqual(try layout.resolve(PresencePoint(x: 10, y: 20, display: 1)), CGPoint(x: 1522, y: 22))
        XCTAssertEqual(try layout.resolve(PresencePoint(x: 10, y: 20)), CGPoint(x: 10, y: 20))
        XCTAssertThrowsError(try layout.resolve(PresencePoint(x: 0, y: 0, display: 2)))
    }

    func testNearestDisplay() {
        XCTAssertEqual(layout.display(nearest: CGPoint(x: 2000, y: 500))?.index, 1)
        XCTAssertEqual(layout.display(nearest: CGPoint(x: -50, y: 10))?.index, 0)
        XCTAssertEqual(layout.display(nearest: CGPoint(x: 1600, y: 1))?.index, 1)
    }

    func testInkResolvesIntoDisplaySpace() throws {
        let shapes = try InkGeometry.resolve([.circle(center: CGPoint(x: 100, y: 100), radius: 10)], display: 1, in: layout)
        XCTAssertEqual(shapes, [.circle(center: CGPoint(x: 1612, y: 102), radius: 10)])
    }
}

final class MotionTests: XCTestCase {
    func testCubicBezierMatchesEndpointsAndIsMonotonic() {
        for curve in [CubicBezier.glide, .ink, .easeInOut] {
            XCTAssertEqual(curve(0), 0)
            XCTAssertEqual(curve(1), 1)
            var previous = 0.0
            for i in 1...100 {
                let y = curve(Double(i) / 100)
                XCTAssertGreaterThanOrEqual(y, previous - 1e-9)
                previous = y
            }
        }
        XCTAssertEqual(CubicBezier.easeInOut(0.5), 0.5, accuracy: 1e-6)
        XCTAssertEqual(CubicBezier(0, 0, 1, 1)(0.3), 0.3, accuracy: 1e-6)
    }

    func testFlightArcsAndLands() {
        let flight = Flight(from: .zero, to: CGPoint(x: 900, y: 0), start: 2)
        XCTAssertEqual(flight.duration, 0.95, accuracy: 1e-9)
        XCTAssertEqual(flight.position(at: 2), .zero)
        XCTAssertEqual(flight.position(at: 2 + flight.duration), CGPoint(x: 900, y: 0))
        // Bows toward the top of the screen (negative y).
        XCTAssertLessThan(flight.position(at: 2 + flight.duration / 2).y, -10)
        XCTAssertEqual(Flight.duration(distance: 0), 0.45)
        XCTAssertEqual(Flight.duration(distance: 10_000), 1.15)
    }

    func testTipGlidesAndLogoTrailsMoreSlowly() {
        var motion = PresenceMotion(at: .zero)
        motion.show(now: 0, reduceMotion: false)
        motion.point(at: CGPoint(x: 600, y: 300), now: 0, reduceMotion: false)
        var time = 0.0
        var logoLagged = false
        while time < 0.6 {
            time += 1.0 / 60
            motion.step(now: time, dt: 1.0 / 60, pointer: nil, reduceMotion: false)
            let tipProgress = motion.tip.x / 600
            let logoProgress = (motion.logo.x - PresenceMotion.followOffset.x) / 600
            if tipProgress - logoProgress > 0.05 { logoLagged = true }
        }
        XCTAssertTrue(logoLagged)
        while time < 3 {
            time += 1.0 / 60
            motion.step(now: time, dt: 1.0 / 60, pointer: nil, reduceMotion: false)
        }
        XCTAssertEqual(motion.tip, CGPoint(x: 600, y: 300))
        XCTAssertFalse(motion.isMoving)
        XCTAssertEqual(motion.logo.x, 622, accuracy: 0.01)
        XCTAssertEqual(motion.logo.y, 318, accuracy: 0.01)
        XCTAssertEqual(motion.opacity, 1)
    }

    func testReduceMotionMovesInstantly() {
        var motion = PresenceMotion(at: .zero)
        motion.show(now: 0, reduceMotion: true)
        XCTAssertEqual(motion.opacity, 1)
        motion.point(at: CGPoint(x: 400, y: 50), now: 0, reduceMotion: true)
        XCTAssertFalse(motion.isMoving)
        XCTAssertEqual(motion.tip, CGPoint(x: 400, y: 50))
        motion.step(now: 0.016, dt: 0.016, pointer: CGPoint(x: 2000, y: 50), reduceMotion: true)
        XCTAssertEqual(motion.logo, CGPoint(x: 422, y: 68))
        // Eyes follow the pointer directly, without easing.
        XCTAssertEqual(motion.eyes, PresenceMotion.gaze(from: motion.logoCenter, toward: CGPoint(x: 2000, y: 50)))
        motion.hide(reduceMotion: true)
        XCTAssertEqual(motion.opacity, 0)
    }

    func testHiddenPointAtRepositionsWithoutFlight() {
        var motion = PresenceMotion(at: .zero)
        motion.point(at: CGPoint(x: 100, y: 100), now: 0, reduceMotion: false)
        XCTAssertFalse(motion.isMoving)
        XCTAssertEqual(motion.tip, CGPoint(x: 100, y: 100))
    }

    func testFadeTakesTwoHundredMilliseconds() {
        var motion = PresenceMotion(at: .zero)
        motion.show(now: 0, reduceMotion: false)
        motion.step(now: 0.1, dt: 0.1, pointer: nil, reduceMotion: false)
        XCTAssertEqual(motion.opacity, 0.5, accuracy: 1e-9)
        motion.step(now: 0.2, dt: 0.1, pointer: nil, reduceMotion: false)
        XCTAssertEqual(motion.opacity, 1, accuracy: 1e-9)
    }

    func testGazeIsBoundedAndPointsTowardThePointer() {
        let center = CGPoint(x: 500, y: 500)
        let far = PresenceMotion.gaze(from: center, toward: CGPoint(x: 100_000, y: -100_000))
        XCTAssertEqual(far.x, 1.2, accuracy: 1e-6)
        XCTAssertEqual(far.y, -1.0, accuracy: 1e-6)
        XCTAssertEqual(PresenceMotion.gaze(from: center, toward: center), .zero)
        XCTAssertGreaterThan(PresenceMotion.gaze(from: center, toward: CGPoint(x: 520, y: 500)).x, 0)
    }

    func testEyesLookAroundOnlyAfterThePointerRests() {
        var motion = PresenceMotion(at: .zero)
        let pointer = CGPoint(x: -1000, y: 0)
        motion.step(now: 0, dt: 0.016, pointer: CGPoint(x: -999, y: 0), reduceMotion: false)
        var time = 0.0
        while time < 1 {
            time += 0.016
            motion.step(now: time, dt: 0.016, pointer: pointer, reduceMotion: false)
        }
        XCTAssertLessThan(motion.eyes.x, -1) // tracking the pointer to the left
        while time < 6.5 {
            time += 0.016
            motion.step(now: time, dt: 0.016, pointer: pointer, reduceMotion: false)
        }
        // 6.5 s into the 9 s look loop is the rest pose.
        XCTAssertEqual(motion.eyes.x, 0, accuracy: 0.05)
        XCTAssertEqual(motion.eyes.y, 0, accuracy: 0.05)
    }

    func testCharacterLoopsMatchTheWebsiteKeyframes() {
        XCTAssertEqual(PresenceMotion.blink(time: 0), 1)
        XCTAssertEqual(PresenceMotion.blink(time: 9 * 0.77), 0.05, accuracy: 1e-9)
        XCTAssertEqual(PresenceMotion.blink(time: 9 * 0.85), 0.05, accuracy: 1e-9)
        XCTAssertEqual(PresenceMotion.blink(time: 9 * 0.5), 1)
        XCTAssertEqual(PresenceMotion.lookAround(time: 9 * 0.2), CGPoint(x: 0, y: -1))
        XCTAssertEqual(PresenceMotion.lookAround(time: 9 * 0.35), CGPoint(x: -1.2, y: 0))
        XCTAssertEqual(PresenceMotion.lookAround(time: 9 * 0.5), CGPoint(x: 1.2, y: 0))
        let peak = PresenceMotion.bob(time: 1.2)
        XCTAssertEqual(peak.offset.x, 3, accuracy: 1e-9)
        XCTAssertEqual(peak.degrees, -4, accuracy: 1e-9)
        XCTAssertEqual(PresenceMotion.bob(time: 0).degrees, 0, accuracy: 1e-9)
    }
}

final class ArtTests: XCTestCase {
    func testMarkPathsFillTheViewBox() throws {
        let upper = try SVGPath.parse(AstridArt.upperArch).boundingBoxOfPath
        let lower = try SVGPath.parse(AstridArt.lowerArch).boundingBoxOfPath
        XCTAssertEqual(upper.minX, 0, accuracy: 0.01)
        XCTAssertEqual(upper.maxX, 20, accuracy: 0.01)
        XCTAssertEqual(upper.minY, 0, accuracy: 0.01)
        XCTAssertEqual(lower.maxY, 15.98, accuracy: 0.01)
        let eye = try SVGPath.parse(AstridArt.leftEye).boundingBoxOfPath
        XCTAssertEqual(eye.minX, 6.009, accuracy: 0.01)
        XCTAssertEqual(eye.maxY, 10.42, accuracy: 0.01)
    }

    func testCursorArrowHandlesImplicitAndRelativeCommands() throws {
        let arrow = try SVGPath.parse(AstridArt.cursorArrow).boundingBoxOfPath
        XCTAssertEqual(arrow, CGRect(x: 7, y: 6, width: 21, height: 23))
        // Same path the web's clickFan(7, 6) emits.
        XCTAssertEqual(AstridArt.cursorClickLines, "M4 6L1.5 6M4.88 3.88L3.11 2.11M7 3L7 0.5M9.12 3.88L10.89 2.11")
        let lines = try SVGPath.parse(AstridArt.cursorClickLines).boundingBoxOfPath
        XCTAssertEqual(lines.minX, 1.5, accuracy: 1e-9)
        XCTAssertEqual(lines.minY, 0.5, accuracy: 1e-9)
        // Every line stays above or level with the tip, clear of the arrow body.
        XCTAssertLessThanOrEqual(lines.maxY, AstridArt.cursorHotspot.y)
    }

    func testRejectsUnsupportedPathData() {
        XCTAssertThrowsError(try SVGPath.parse("10 10"))
        XCTAssertThrowsError(try SVGPath.parse("M0 0 Q1 1 2 2"))
        XCTAssertThrowsError(try SVGPath.parse("M0 0L"))
    }

    func testInkIsDeterministicAndSurroundsTheTarget() {
        let center = CGPoint(x: 300, y: 200)
        let a = InkGeometry.circle(center: center, radius: 50)
        XCTAssertEqual(a, InkGeometry.circle(center: center, radius: 50))
        let box = a.boundingBoxOfPath
        XCTAssertTrue(box.contains(CGRect(x: 260, y: 165, width: 80, height: 70)))
        XCTAssertLessThan(box.width, 140)
        let arrow = InkGeometry.arrow(from: .zero, to: CGPoint(x: 100, y: 0)).boundingBoxOfPath
        XCTAssertEqual(arrow.maxX, 100, accuracy: 0.01)
        XCTAssertGreaterThan(arrow.height, 10) // the head opens on both sides
    }

    func testInkCanvasLimit() throws {
        var canvas = InkCanvas()
        try canvas.admit(PresenceProtocol.Limits.maxInkShapes)
        XCTAssertThrowsError(try canvas.admit(1))
        canvas.clear()
        XCTAssertNoThrow(try canvas.admit(1))
    }

    func testSubtitleReadingTimeIsClamped() {
        XCTAssertEqual(Subtitles.readingTime("hi"), 2.5)
        XCTAssertEqual(Subtitles.readingTime(String(repeating: "a", count: 1000)), 9)
    }
}

final class DemoTests: XCTestCase {
    func testDemoIsValidAndVisitsEveryDisplay() throws {
        let layout = DisplayLayout.fromCocoa(frames: [
            CGRect(x: 0, y: 0, width: 1512, height: 982),
            CGRect(x: -1920, y: 0, width: 1920, height: 1080),
        ])
        let events = DemoScript.events(for: layout)
        for event in events {
            XCTAssertEqual(try PresenceProtocol.parse(line: PresenceProtocol.encode(event)), event)
            if case let .pointAt(point, _) = event {
                let global = try layout.resolve(point)
                XCTAssertNotNil(layout.displays.first { $0.frame.contains(global) }, "\(point) is off screen")
            }
        }
        let displays = Set(events.compactMap { event -> Int? in
            if case let .pointAt(point, _) = event { return point.display }
            return nil
        })
        XCTAssertEqual(displays, [0, 1])
        XCTAssertTrue(DemoScript.events(for: DisplayLayout(displays: [])).isEmpty)
    }
}

final class CaptureProtocolTests: XCTestCase {
    func testCaptureEventParsesAndRoundTrips() throws {
        let event = try PresenceProtocol.parse(line: #"{"v":1,"type":"capture","request_id":"a1-b2","display":0}"#)
        XCTAssertEqual(event, .capture(requestId: "a1-b2", display: 0))
        XCTAssertEqual(try PresenceProtocol.parse(line: PresenceProtocol.encode(event)), event)
        for bad in [
            #"{"v":1,"type":"capture","display":0}"#,
            #"{"v":1,"type":"capture","request_id":"","display":0}"#,
            #"{"v":1,"type":"capture","request_id":"a b","display":0}"#,
            #"{"v":1,"type":"capture","request_id":"x"}"#,
            #"{"v":1,"type":"capture","request_id":"\#(String(repeating: "a", count: 65))","display":0}"#,
        ] {
            XCTAssertThrowsError(try PresenceProtocol.parse(line: bad), bad)
        }
    }

    func testFrameLimitsMatchADR0022() {
        XCTAssertNil(FrameLimits.check(width: 1512, height: 982, bytes: 900_000))
        XCTAssertEqual(FrameLimits.check(width: 2561, height: 100, bytes: 0), "display_exceeds_frame_limits")
        XCTAssertEqual(FrameLimits.check(width: 2560, height: 1601, bytes: 0), "display_exceeds_frame_limits")
        XCTAssertNil(FrameLimits.check(width: 2560, height: 1600, bytes: 0))
        XCTAssertEqual(FrameLimits.check(width: 1512, height: 982, bytes: 1024 * 1024 + 1), "frame_png_exceeds_1mib")
        XCTAssertEqual(FrameLimits.check(width: 0, height: 982, bytes: 0), "display_unavailable")
    }

    func testSyntheticTargetsAreInsideTheFrameAndDisjoint() {
        let targets = SyntheticTargets.all(width: 1512, height: 982)
        XCTAssertEqual(targets.map(\.label), ["Open", "Share", "Export", "Settings", "Delete"])
        let frame = CGRect(x: 0, y: 0, width: 1512, height: 982)
        for (i, a) in targets.enumerated() {
            XCTAssertTrue(frame.contains(a.box))
            for b in targets[(i + 1)...] { XCTAssertFalse(a.box.intersects(b.box)) }
        }
        XCTAssertEqual(targets[2].box, CGRect(x: 1150, y: 159, width: 120, height: 36))
    }
}

final class TimerTests: XCTestCase {
    let now = Date(timeIntervalSince1970: 1_800_000_000)

    func testTimerEventsParseAndRoundTrip() throws {
        let events: [PresenceEvent] = [
            .timerStart(id: "tea", seconds: 180, label: "Tea"),
            .timerCancel(id: "tea"),
            .alarmSet(id: "wake-1", at: Date(timeIntervalSince1970: 1_800_003_600), label: "Wake up"),
            .timersList,
        ]
        for event in events {
            XCTAssertEqual(try PresenceProtocol.parse(line: PresenceProtocol.encode(event)), event)
        }
        XCTAssertEqual(try PresenceProtocol.parse(line: #"{"v":1,"type":"alarm_set","id":"a","at":"2027-01-15T07:30:00+01:00"}"#),
                       .alarmSet(id: "a", at: PresenceProtocol.isoDate("2027-01-15T06:30:00Z")!, label: ""))
        for bad in [
            #"{"v":1,"type":"timer_start","id":"t","label":"x"}"#,
            #"{"v":1,"type":"timer_start","id":"bad id","seconds":5}"#,
            #"{"v":1,"type":"alarm_set","id":"a","at":"tomorrow 7am"}"#,
            #"{"v":1,"type":"alarm_set","id":"a","at":"2027-01-15T07:30:00"}"#,
        ] {
            XCTAssertThrowsError(try PresenceProtocol.parse(line: bad), bad)
        }
    }

    func testLimitsOrderingAndSingleFiring() throws {
        var store = TimerStore()
        let tea = try store.startTimer(id: "tea", seconds: 180, label: "  Tea  ", now: now)
        XCTAssertEqual(tea.label, "Tea")
        _ = try store.setAlarm(id: "wake", at: now.addingTimeInterval(60), label: "Wake", now: now)
        XCTAssertEqual(store.entries.map(\.id), ["wake", "tea"])
        XCTAssertThrowsError(try store.startTimer(id: "tea", seconds: 5, label: "", now: now))
        XCTAssertThrowsError(try store.startTimer(id: "x", seconds: 0, label: "", now: now))
        XCTAssertThrowsError(try store.startTimer(id: "x", seconds: 86_401, label: "", now: now))
        XCTAssertThrowsError(try store.setAlarm(id: "x", at: now.addingTimeInterval(-1), label: "", now: now))
        XCTAssertThrowsError(try store.setAlarm(id: "x", at: now.addingTimeInterval(8 * 86_400), label: "", now: now))
        XCTAssertThrowsError(try store.startTimer(id: "x", seconds: 5, label: String(repeating: "a", count: 121), now: now))
        XCTAssertThrowsError(try store.startTimer(id: "x", seconds: 5, label: "a\u{7}b", now: now))
        XCTAssertEqual(store.takeDue(now: now.addingTimeInterval(59)), [])
        XCTAssertEqual(store.takeDue(now: now.addingTimeInterval(200)).map(\.id), ["wake", "tea"])
        XCTAssertEqual(store.takeDue(now: now.addingTimeInterval(300)), [], "each entry fires once")
        for i in 0..<TimerStore.maxActive { _ = try store.startTimer(id: "t\(i)", seconds: 10, label: "", now: now) }
        XCTAssertThrowsError(try store.startTimer(id: "over", seconds: 10, label: "", now: now))
        XCTAssertNotNil(store.cancel(id: "t3"))
        XCTAssertNil(store.cancel(id: "t3"))
    }

    func testPersistenceIsOwnerOnlyAndLateTimersFireOnLoad() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let url = dir.appendingPathComponent("Astrid/timers.json")
        defer { try? FileManager.default.removeItem(at: dir) }
        var store = TimerStore()
        _ = try store.startTimer(id: "tea", seconds: 60, label: "Tea", now: now)
        try store.save(to: url)
        let mode = try FileManager.default.attributesOfItem(atPath: url.path)[.posixPermissions] as? Int
        XCTAssertEqual(mode, 0o600)
        var loaded = TimerStore.load(from: url)
        XCTAssertEqual(loaded, store)
        let late = now.addingTimeInterval(60 + 600)
        let fired = loaded.takeDue(now: late)
        XCTAssertEqual(fired.map(\.id), ["tea"])
        XCTAssertEqual(timerFiredText(fired[0], now: late), "Timer done: Tea (late 10 min)")
        try Data("garbage".utf8).write(to: url)
        XCTAssertEqual(TimerStore.load(from: url), TimerStore())
    }
}

final class CursorStyleTests: XCTestCase {
    func testPointAtCarriesAValidatedCursorStyle() throws {
        XCTAssertEqual(
            try PresenceProtocol.parse(line: #"{"v":1,"type":"point_at","x":1,"y":2,"display":0,"cursor":"text"}"#),
            .pointAt(PresencePoint(x: 1, y: 2, display: 0), cursor: .text))
        XCTAssertEqual(
            try PresenceProtocol.parse(line: #"{"v":1,"type":"point_at","x":1,"y":2}"#),
            .pointAt(PresencePoint(x: 1, y: 2), cursor: nil))
        for line in [
            #"{"v":1,"type":"point_at","x":1,"y":2,"cursor":"lasso"}"#,
            #"{"v":1,"type":"point_at","x":1,"y":2,"cursor":"Text select"}"#,
            #"{"v":1,"type":"point_at","x":1,"y":2,"cursor":3}"#,
        ] {
            XCTAssertThrowsError(try PresenceProtocol.parse(line: line), line)
        }
        for style in CursorStyle.allCases {
            let event = PresenceEvent.pointAt(PresencePoint(x: 5, y: 6, display: 1), cursor: style)
            XCTAssertEqual(try PresenceProtocol.parse(line: PresenceProtocol.encode(event)), event)
        }
        // No cursor encodes exactly as before, so v1 readers see no change.
        XCTAssertFalse(PresenceProtocol.encode(.pointAt(PresencePoint(x: 5, y: 6))).contains("cursor"))
    }

    func testWaveIsAnEvent() throws {
        XCTAssertEqual(try PresenceProtocol.parse(line: #"{"v":1,"type":"wave"}"#), .wave)
        XCTAssertEqual(try PresenceProtocol.parse(line: PresenceProtocol.encode(.wave)), .wave)
    }

    func testEveryStyleHasDrawableArtOnTheGrid() throws {
        let grid = CGRect(x: 0, y: 0, width: AstridArt.cursorGrid, height: AstridArt.cursorGrid)
        XCTAssertEqual(CursorStyle.default, .arrowClick)
        XCTAssertEqual(Set(CursorStyle.allCases.map(\.docsName)).count, CursorStyle.allCases.count)
        for style in CursorStyle.allCases {
            let art = AstridArt.cursor(style)
            XCTAssertTrue(grid.contains(art.hotspot), "\(style) hotspot")
            XCTAssertFalse(art.parts.isEmpty, "\(style)")
            for part in art.parts {
                XCTAssertTrue(part.fill != nil || part.stroke != nil, "\(style) part paints nothing")
                var scale = CGAffineTransform(scaleX: part.scale, y: part.scale)
                let path = try XCTUnwrap(try SVGPath.parse(part.path).copy(using: &scale), "\(style)")
                XCTAssertFalse(path.isEmpty, "\(style)")
                XCTAssertTrue(grid.insetBy(dx: -1, dy: -1).contains(path.boundingBoxOfPath), "\(style) leaves the grid: \(path.boundingBoxOfPath)")
            }
        }
    }

    func testArcsMatchTheirCircles() throws {
        // A full circle as two half arcs, and a half arc, as the art uses them.
        let circle = try SVGPath.parse("M3 13a10 10 0 1 0 20 0a10 10 0 1 0 -20 0Z").boundingBoxOfPath
        XCTAssertEqual(circle.minX, 3, accuracy: 0.01)
        XCTAssertEqual(circle.maxX, 23, accuracy: 0.01)
        XCTAssertEqual(circle.minY, 3, accuracy: 0.01)
        XCTAssertEqual(circle.maxY, 23, accuracy: 0.01)
        // "a2 2 0 0 1 4 0" from (10, 7.5) bulges up to y = 5.5 (sweep 1, y down).
        let knuckle = try SVGPath.parse("M10 7.5a2 2 0 0 1 4 0").boundingBoxOfPath
        XCTAssertEqual(knuckle.minY, 5.5, accuracy: 0.01)
        XCTAssertEqual(knuckle.maxY, 7.5, accuracy: 0.01)
        // Radii too small for the endpoints scale up instead of failing.
        let scaled = try SVGPath.parse("M0 0a1 1 0 0 1 10 0").boundingBoxOfPath
        XCTAssertEqual(scaled.width, 10, accuracy: 0.01)
        XCTAssertThrowsError(try SVGPath.parse("M0 0a1 1 0 0 1 10"))
    }
}

final class OperatorTests: XCTestCase {
    func testActionsAreBoundedLikeTheRuntime() throws {
        XCTAssertEqual(try OperatorAction.parse(["kind": "click"]), .click)
        XCTAssertEqual(try OperatorAction.parse(["kind": "type", "text": "Q3 report"]), .type("Q3 report"))
        XCTAssertEqual(try OperatorAction.parse(["kind": "key", "key": "enter"]), .key("enter"))
        XCTAssertEqual(try OperatorAction.parse(["kind": "scroll", "direction": "down", "notches": 3]), .scroll(down: true, notches: 3))
        for bad: [String: Any] in [
            ["kind": "drag"],
            ["kind": "type", "text": ""],
            ["kind": "type", "text": "a\nb"],
            ["kind": "type", "text": String(repeating: "x", count: 201)],
            ["kind": "key", "key": "cmd+q"],
            ["kind": "scroll", "direction": "left", "notches": 1],
            ["kind": "scroll", "direction": "up", "notches": 11],
        ] {
            XCTAssertThrowsError(try OperatorAction.parse(bad), "\(bad)")
        }
        // Same key set as the runtime's operator::KEYS.
        XCTAssertEqual(Set(OperatorAction.keyCodes.keys), ["enter", "tab", "escape", "space", "up", "down", "left", "right"])
    }

    private func rgba(width: Int, height: Int, square: CGRect? = nil) -> [UInt8] {
        var bytes = [UInt8](repeating: 40, count: width * height * 4)
        if let square {
            for y in Int(square.minY)..<Int(square.maxY) {
                for x in Int(square.minX)..<Int(square.maxX) {
                    let i = (y * width + x) * 4
                    bytes[i] = 250; bytes[i + 1] = 250; bytes[i + 2] = 250
                }
            }
        }
        return bytes
    }

    func testFreshnessComparesOnlyTheAreaAroundTheTarget() {
        let (w, h) = (320, 200)
        let before = Fingerprint(rgba: rgba(width: w, height: h), width: w, height: h, bytesPerRow: w * 4)
        XCTAssertEqual(before.width, 80)
        XCTAssertEqual(before.height, 50)
        // A change far from the target (a clock ticking in a corner) is fine.
        let corner = Fingerprint(rgba: rgba(width: w, height: h, square: CGRect(x: 280, y: 0, width: 40, height: 20)), width: w, height: h, bytesPerRow: w * 4)
        XCTAssertTrue(before.matches(corner, around: CGPoint(x: 60, y: 150)))
        // A dialog appearing over the target is not.
        let dialog = Fingerprint(rgba: rgba(width: w, height: h, square: CGRect(x: 30, y: 120, width: 80, height: 60)), width: w, height: h, bytesPerRow: w * 4)
        XCTAssertFalse(before.matches(dialog, around: CGPoint(x: 60, y: 150)))
        // Another display size never matches.
        let other = Fingerprint(rgba: rgba(width: 160, height: 100), width: 160, height: 100, bytesPerRow: 640)
        XCTAssertFalse(before.matches(other, around: CGPoint(x: 10, y: 10)))
    }
}

final class FrameEncodingTests: XCTestCase {
    /// A photo-like frame: gradients plus small per-pixel noise.
    private func photo(width: Int, height: Int) -> [UInt8] {
        var rgba = [UInt8](repeating: 255, count: width * height * 4)
        var seed: UInt64 = 7
        func noise() -> Int { seed = seed &* 6364136223846793005 &+ 1442695040888963407; return Int(seed >> 61) - 4 }
        for y in 0..<height {
            for x in 0..<width {
                let i = (y * width + x) * 4
                rgba[i] = UInt8(clamping: x * 255 / width + noise())
                rgba[i + 1] = UInt8(clamping: y * 255 / height + noise())
                rgba[i + 2] = UInt8(clamping: (x + y) * 128 / (width + height) + noise())
            }
        }
        return rgba
    }

    func testRealScreensAreFittedUnderTheLimitAtFullSize() throws {
        let (w, h) = (400, 300)
        let rgba = photo(width: w, height: h)
        let full = try XCTUnwrap(FrameEncoding.fit(rgba: rgba, width: w, height: h, bytesPerRow: w * 4, limit: 10_000_000))
        XCTAssertEqual(full.bits, 8)
        // A tighter limit is met by quantizing, never by resizing.
        let tight = try XCTUnwrap(FrameEncoding.fit(rgba: rgba, width: w, height: h, bytesPerRow: w * 4, limit: full.png.count / 2))
        XCTAssertLessThan(tight.bits, 8)
        XCTAssertLessThanOrEqual(tight.png.count, full.png.count / 2)
        let source = try XCTUnwrap(CGImageSourceCreateWithData(tight.png as CFData, nil))
        let props = try XCTUnwrap(CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any])
        XCTAssertEqual(props[kCGImagePropertyPixelWidth] as? Int, w)
        XCTAssertEqual(props[kCGImagePropertyPixelHeight] as? Int, h)
        XCTAssertEqual(props[kCGImagePropertyHasAlpha] as? Bool ?? false, false)
        // Nothing fits an impossible limit.
        XCTAssertNil(FrameEncoding.fit(rgba: rgba, width: w, height: h, bytesPerRow: w * 4, limit: 100))
    }

    func testQuantizingKeepsBlackAndWhite() {
        let rgba: [UInt8] = [0, 255, 128, 255, 255, 0, 17, 255]
        for bits in FrameEncoding.steps {
            let rgb = FrameEncoding.rgb(rgba, width: 2, height: 1, bytesPerRow: 8, bits: bits)
            XCTAssertEqual(rgb[0], 0)
            XCTAssertEqual(rgb[1], 255)
            XCTAssertEqual(rgb[3], 255)
            XCTAssertEqual(rgb[4], 0)
        }
        XCTAssertEqual(FrameEncoding.rgb(rgba, width: 2, height: 1, bytesPerRow: 8, bits: 8), [0, 255, 128, 255, 0, 17])
    }
}
