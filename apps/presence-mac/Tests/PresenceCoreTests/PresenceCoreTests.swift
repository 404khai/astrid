import CoreGraphics
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
        XCTAssertEqual(lower.maxY, 16, accuracy: 0.01)
        let eye = try SVGPath.parse(AstridArt.leftEye).boundingBoxOfPath
        XCTAssertEqual(eye.minX, 6.009, accuracy: 0.01)
        XCTAssertEqual(eye.maxY, 10.43, accuracy: 0.01)
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
        XCTAssertThrowsError(try SVGPath.parse("M0 0 A1 1 0 0 0 2 2"))
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
            if case let .pointAt(point) = event {
                let global = try layout.resolve(point)
                XCTAssertNotNil(layout.displays.first { $0.frame.contains(global) }, "\(point) is off screen")
            }
        }
        let displays = Set(events.compactMap { event -> Int? in
            if case let .pointAt(point) = event { return point.display }
            return nil
        })
        XCTAssertEqual(displays, [0, 1])
        XCTAssertTrue(DemoScript.events(for: DisplayLayout(displays: [])).isEmpty)
    }
}
