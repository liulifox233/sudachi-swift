// swift-tools-version:5.5
// The root Swift package is rendered by swift/scripts/render-root-package.sh.

import Foundation
import PackageDescription

let packageName = "SudachiSwift"
let binaryTargetName = "sudachi_swiftFFI"
let repositoryURL = "https://github.com/liulifox233/sudachi-swift"
let releaseTag = "v0.6.11-a1"
let binaryArtifactName = "SudachiSwiftFFI.xcframework.zip"
let remoteBinaryArtifactURL = "\(repositoryURL)/releases/download/\(releaseTag)/\(binaryArtifactName)"
let remoteBinaryArtifactChecksum = "0000000000000000000000000000000000000000000000000000000000000000"

// Local development and CI can point the package at a freshly built xcframework.
let localBinaryArtifactPath = ProcessInfo.processInfo.environment["SUDACHI_SWIFT_LOCAL_BINARY_PATH"]

let binaryTarget: Target = {
    if let localBinaryArtifactPath, !localBinaryArtifactPath.isEmpty {
        return .binaryTarget(
            name: binaryTargetName,
            path: localBinaryArtifactPath
        )
    }

    return .binaryTarget(
        name: binaryTargetName,
        url: remoteBinaryArtifactURL,
        checksum: remoteBinaryArtifactChecksum
    )
}()

let package = Package(
    name: packageName,
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15),
    ],
    products: [
        .library(
            name: packageName,
            targets: [packageName]
        )
    ],
    targets: [
        binaryTarget,
        .target(
            name: packageName,
            dependencies: [
                .target(name: binaryTargetName)
            ],
            path: "swift/generated/sources",
            sources: [
                "sudachi_swift.swift"
            ]
        ),
    ]
)
