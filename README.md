# mapping-serde

serde-like framework for serialization and deserialization of Java deobfuscation mappings.

## Supported formats

- [Tiny1](https://wiki.fabricmc.net/documentation:tiny)
- [Tiny2](https://wiki.fabricmc.net/documentation:tiny2)
- [Enigma](https://wiki.fabricmc.net/documentation:enigma_mappings)
- Proguard _(deserialization only)_

## Utilities

- Element/Value type for generic parsing of mappings
- Nest a flat mapping into a tree-style one
- Flatten a tree mapping into a flat one

## Sub Crates

- `java-mapping-serde`: The framework itself.
- `java-mapping-serde-element`: Generalized in-memory representation, similiar to `serde-value`.
- `java-mapping-serde-util`: Utilities essential for mapping transforming.
- `java-mapping-serde-io-util`: I/O utilities for reader and writer implementations. Useful for users as well.

- `java-mapping-serde-tiny1`: Tiny1 format support.
- `java-mapping-serde-tiny2`: Tiny2 format support.
- `java-mapping-serde-enigma`: Enigma format support.
- `java-mapping-serde-proguard`: Proguard format support. (read-only)
