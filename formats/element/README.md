# java-mapping-serde-element

Generalized, fully-owned in-memory representation of Java deobfuscation mappings, in the spirit of `serde-value`.

Any mapping deserializer can build an `Element` tree, which can then be inspected, transformed, or re-serialized without knowing the source format.

## Usage

```rust
let elements = mapping_serde_element::deserialize_from(deserializer)?;
```

`Element` is an enum of `Class`, `Field`, `Method`, `MethodArg`, `MethodVar`, and `Comment`.
