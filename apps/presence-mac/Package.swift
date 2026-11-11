// swift-tools-version:5.9
import PackageDescription

// Astrid's macOS presence host (V2-01, ADR 0026). PresenceCore holds the
// presence protocol, motion and geometry with no AppKit dependency so it can
// be tested headlessly; AstridPresence is the thin AppKit overlay client.
let package = Package(
    name: "AstridPresence",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "astrid-presence", targets: ["AstridPresence"]),
        .library(name: "PresenceCore", targets: ["PresenceCore"]),
    ],
    targets: [
        .target(name: "PresenceCore"),
        .executableTarget(name: "AstridPresence", dependencies: ["PresenceCore"]),
        .testTarget(name: "PresenceCoreTests", dependencies: ["PresenceCore"]),
    ]
)
