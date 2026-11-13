// swift-tools-version:5.9
import PackageDescription

// Astrid's macOS presence host (V2-01, ADR 0026). PresenceCore holds the
// presence protocol, motion and geometry with no AppKit dependency so it can
// be tested headlessly; AstridPresence is the thin AppKit overlay client.
// AstridWebUI serves the built web UI to web views here and in the desktop
// app (apps/astrid-desktop, ADR 0037).
let package = Package(
    name: "AstridPresence",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "astrid-presence", targets: ["AstridPresence"]),
        .library(name: "PresenceCore", targets: ["PresenceCore"]),
        .library(name: "AstridWebUI", targets: ["AstridWebUI"]),
    ],
    targets: [
        .target(name: "PresenceCore"),
        .target(name: "AstridWebUI"),
        .executableTarget(name: "AstridPresence", dependencies: ["PresenceCore", "AstridWebUI"]),
        .testTarget(name: "PresenceCoreTests", dependencies: ["PresenceCore"]),
    ]
)
