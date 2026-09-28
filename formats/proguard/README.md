# java-mapping-serde-proguard

[ProGuard](https://www.guardsquare.com/manual/tools/retrace) mapping format support for `java-mapping-serde`.

Deserialization only.

## Usage

```rust
let elements = mapping_serde_proguard::from_str(text, "srcNs", "dstNs")?;
```
