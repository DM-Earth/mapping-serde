# java-mapping-serde-enigma

[Enigma](https://wiki.fabricmc.net/documentation:enigma_mappings) format support for `java-mapping-serde`.

Supports both deserialization and serialization, from a single file or a directory.

## Usage

```rust
let elements = mapping_serde_enigma::from_str(text, "srcNs", "dstNs")?;
let text = mapping_serde_enigma::to_string(&elements)?;
let elements = mapping_serde_enigma::from_directory("mappings/", "srcNs", "dstNs")?;
```
