# java-mapping-serde-tiny2

[Tiny v2](https://wiki.fabricmc.net/documentation:tiny2) format support for `java-mapping-serde`.

Supports both deserialization and serialization, including multiple destination namespaces, properties, and escaped names.

## Usage

```rust
let elements = mapping_serde_tiny2::from_str(text)?;
let text = mapping_serde_tiny2::to_string(&elements, "srcNs", &["dstNs"], &[])?;
```
