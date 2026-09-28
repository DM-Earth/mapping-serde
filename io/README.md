# java-mapping-serde-io-util

I/O utilities for implementing readers and writers of line- and column-based mapping formats.

Useful on its own for anyone parsing mapping files, not just for this workspace.

## Provided

- `ColumnRead` and adapters (`ColumnReadAdapter`, `ColumnReader`) for reading tab- or space-separated columns.
- `SliceReader` and `IoReader` over byte slices and `BufRead`.
- `MaybeBorrowed` / `MaybeMut` for references that may or may not outlive the parse.
- `SmolCowStr`, behind the `smol-str` feature.
