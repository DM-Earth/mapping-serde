# java-mapping-serde-util

Utilities for transforming mappings, plus helpers for writing `mapping-serde` deserializers.

## Provided

- `DeserializerExt` — extension methods for deserializers.
- `RefVisitor` — a visitor for borrowed values.
- `Nest` — nest a flat mapping into a tree.
- `Flatten` — flatten a tree mapping into a flat one.

`Nest` and `Flatten` require the `translate` feature.
