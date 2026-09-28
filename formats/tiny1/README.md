# java-mapping-serde-tiny1

[Tiny v1](https://wiki.fabricmc.net/documentation:tiny) format support for `java-mapping-serde`.

Supports both deserialization and serialization. Because Tiny1 is a flat format, there are three ways to read it:

- `from_str_indexed` — index the whole file, then build a tree. Most conservative, slowest.
- `from_str_fast` — treat the file as tree-style. Best for generated mappings like `intermediary`.
- `StreamDeserializer` — visit each entry directly, for low-level access.

## Usage

```rust
let elements = mapping_serde_tiny1::from_str_indexed(text)?;
let elements = mapping_serde_tiny1::from_str_fast(text)?;
```
