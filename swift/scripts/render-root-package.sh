#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage: render-root-package.sh [--tag TAG] [--checksum SHA256] [--repository-url URL] [--asset-name NAME]

Renders the repository root Package.swift for the SwiftPM package that Xcode imports
via the repository URL. The release workflow rewrites the tag/checksum before tagging.
EOF
}

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
package_file="${repo_root}/Package.swift"
default_tag="v$(sed -n 's/^version = "\(.*\)"$/\1/p' "${repo_root}/Cargo.toml" | head -n 1)"
default_checksum="0000000000000000000000000000000000000000000000000000000000000000"

normalize_git_url() {
    local raw_url="$1"

    case "${raw_url}" in
        git@github.com:*)
            raw_url="https://github.com/${raw_url#git@github.com:}"
            ;;
        ssh://git@github.com/*)
            raw_url="https://github.com/${raw_url#ssh://git@github.com/}"
            ;;
        https://github.com/*)
            ;;
        *)
            echo "${raw_url}"
            return 0
            ;;
    esac

    echo "${raw_url%.git}"
}

discover_repository_url() {
    if [[ -n "${SUDACHI_SWIFT_REPOSITORY_URL:-}" ]]; then
        echo "${SUDACHI_SWIFT_REPOSITORY_URL}"
        return 0
    fi

    if [[ -n "${GITHUB_REPOSITORY:-}" ]]; then
        local server_url="${GITHUB_SERVER_URL:-https://github.com}"
        echo "${server_url}/${GITHUB_REPOSITORY}"
        return 0
    fi

    if git -C "${repo_root}" remote get-url liulifox233 >/dev/null 2>&1; then
        normalize_git_url "$(git -C "${repo_root}" remote get-url liulifox233)"
        return 0
    fi

    if git -C "${repo_root}" remote get-url origin >/dev/null 2>&1; then
        normalize_git_url "$(git -C "${repo_root}" remote get-url origin)"
        return 0
    fi

    sed -n 's/^repository = "\(.*\)"$/\1/p' "${repo_root}/Cargo.toml" | head -n 1
}

default_repository_url="$(discover_repository_url)"

tag_name="${default_tag}"
checksum="${default_checksum}"
repository_url="${default_repository_url}"
asset_name="SudachiSwiftFFI.xcframework.zip"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --tag)
            tag_name="$2"
            shift 2
            ;;
        --checksum)
            checksum="$2"
            shift 2
            ;;
        --repository-url)
            repository_url="$2"
            shift 2
            ;;
        --asset-name)
            asset_name="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ ! "${checksum}" =~ ^[0-9a-f]{64}$ ]]; then
    echo "Checksum must be a lowercase 64-character SHA-256 hex string." >&2
    exit 1
fi

cat >"${package_file}" <<EOF
// swift-tools-version:5.5
// The root Swift package is rendered by swift/scripts/render-root-package.sh.

import Foundation
import PackageDescription

let packageName = "SudachiSwift"
let binaryTargetName = "sudachi_swiftFFI"
let repositoryURL = "${repository_url}"
let releaseTag = "${tag_name}"
let binaryArtifactName = "${asset_name}"
let remoteBinaryArtifactURL = "\\(repositoryURL)/releases/download/\\(releaseTag)/\\(binaryArtifactName)"
let remoteBinaryArtifactChecksum = "${checksum}"

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
EOF
