# sudachi.rs Swift bindings

This crate is the Rust source for the Swift bindings of `sudachi.rs`.

The published Swift package artifacts are generated in CI/CD with `cargo swift`.
The repository root now contains the consumable SwiftPM entrypoint for Xcode:

- `/Package.swift` is the manifest Xcode resolves from the repository URL.
- `swift/generated/sources/sudachi_swift.swift` is the committed UniFFI-generated wrapper source.
- `SudachiSwiftFFI.xcframework.zip` is attached to GitHub Releases and referenced as a remote binary target.

Transient build outputs such as `swift/SudachiSwift/`, `swift/.build/`, and `swift/artifacts/`
remain uncommitted.

## Public Swift API

The generated Swift package exposes these primary types:

- `SudachiDictionary`
- `SudachiTokenizer`
- `SudachiMorphemeList`
- `SudachiSplitMode`
- `SudachiMorpheme`
- `SudachiWordInfo`
- `SudachiPosMatcher`
- `SudachiProjection`
- `SudachiInfoField`
- `SudachiError`

Usage shape:

```swift
let dictionary = try SudachiDictionary(
    systemDictPath: "/path/to/system.dic",
    configPath: nil,
    resourceDir: "/path/to/resources"
)

let tokenizer = dictionary.createTokenizer(
    mode: .c,
    fields: nil,
    projection: nil
)

let morphemes = try tokenizer.tokenize(text: "東京都")
let first = try morphemes.morphemeAt(index: 0)
print(first.surface())

let split = try first.split(mode: .a, addSingle: nil)
let nounMatcher = try dictionary.makePosMatcher(patterns: [
    SudachiPartialPartOfSpeech(level1: "名詞")
])
```

`systemDictPath` is always required for the Swift bindings. `configPath` and `resourceDir` are optional and map directly to the underlying Rust configuration behavior.

The Swift API tracks the Python binding's object model for core tokenizer features:

- `tokenize` returns `SudachiMorphemeList`
- morphemes are first-class objects and support `split`
- `wordInfo`, dictionary `lookup`, POS lookup, and POS matcher operations are exposed
- `fields` and `projection` are available as typed Swift enums rather than Python string sets

Python ecosystem-specific features such as `pre_tokenizer` are intentionally not part of the Swift bindings.

## Packaging

Local packaging example:

```bash
cd swift
cargo swift package --platforms ios macos --name SudachiSwift --accept-all
```

For repository-root SwiftPM validation during local development:

```bash
cargo swift package --platforms ios macos --name SudachiSwift --accept-all
SUDACHI_SWIFT_LOCAL_BINARY_PATH=swift/SudachiSwift/sudachi_swiftFFI.xcframework swift build
```

Release publishing is handled by `.github/workflows/swift-release.yml`. It builds the
xcframework, computes the SwiftPM checksum, rewrites the root `Package.swift`, and uploads
`SudachiSwiftFFI.xcframework.zip` to the matching GitHub Release tag.
