// swift-tools-version:5.9
import PackageDescription

// Astrid's desktop chat app (ADR 0037): a normal Dock app, separate from the
// presence host. It targets macOS 26 so it links against the Liquid Glass
// SDK; SwiftPM stamps the binary's SDK version from the deployment target,
// and AppKit picks its system look from that stamp. DesktopCore (chat store,
// titles, runtime-child protocol) has no AppKit dependency and is tested
// headlessly. The web UI file server is shared with the presence host.
let package = Package(
    name: "AstridDesktop",
    platforms: [.macOS("26.0")],
    products: [
        .executable(name: "astrid-desktop", targets: ["AstridDesktop"]),
    ],
    dependencies: [
        .package(path: "../presence-mac"),
    ],
    targets: [
        .target(name: "DesktopCore"),
        .executableTarget(name: "AstridDesktop", dependencies: [
            "DesktopCore",
            .product(name: "AstridWebUI", package: "presence-mac"),
        ]),
        .testTarget(name: "DesktopCoreTests", dependencies: ["DesktopCore"]),
    ]
)
