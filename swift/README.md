# sudachi.rs Swift bindings

This crate is the Rust source for the Swift bindings of `sudachi.rs`.

The published Swift package artifacts are generated in CI/CD with `cargo swift`.
Generated outputs such as `swift/generated/` and `swift/SudachiSwift/` are intentionally not committed to the repository.

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
cargo swift package -n SudachiSwift -p macos@10_15 -y
```

The exact release packaging flow should continue to live in CI/CD.
