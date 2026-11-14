import DesktopCore
import XCTest

final class AskEnvironmentTests: XCTestCase {
    func testABareLaunchdPathStillFindsNode() throws {
        let home = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).path
        let bin = "\(home)/.nvm/versions/node/v24.16.0/bin"
        let old = "\(home)/.nvm/versions/node/v9.0.0/bin"
        for dir in [bin, old] {
            try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
            FileManager.default.createFile(atPath: "\(dir)/node", contents: Data(), attributes: [.posixPermissions: 0o755])
        }
        defer { try? FileManager.default.removeItem(atPath: home) }

        let env = AskChild.environment(base: ["PATH": "/usr/bin:/bin"], home: home)
        XCTAssertEqual(env["ASTRID_NODE"], "\(bin)/node", "the newest nvm Node, by version not by name")
        XCTAssertTrue(env["PATH"]!.hasPrefix("/usr/bin:/bin:\(bin)"))
        XCTAssertEqual(AskChild.environment(base: ["PATH": "/usr/bin", "ASTRID_NODE": "/x/node"], home: home)["ASTRID_NODE"], "/x/node")
    }
}
