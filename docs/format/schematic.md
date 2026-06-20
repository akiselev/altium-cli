# Schematic Files

## Records

The complete record discriminant catalog is [`SchRecordType`](../../crates/altium-format-types/src/sch.rs). Unknown discriminants are errors; there is no unknown-record retention variant.

SchDoc dispatch is implemented in [`schdoc/dispatch.rs`](../../crates/altium-format/src/schdoc/dispatch.rs). SchLib dispatch is implemented in [`schlib.rs`](../../crates/altium-format/src/schlib.rs). Both consume all parameters after typed parsing.

When serializing a record type of 256 or greater, [`insert_record_key`](../../crates/altium-format/src/sch_records.rs) writes `RECORD=254` and `RECORDEX=<actual>`. Lower values are written directly as `RECORD=<value>`.

## SchDoc structure

SchDoc uses a flat record list in `/FileHeader`. Ownership is represented by `OWNERINDEX`; the high-level API reconstructs the tree and the write path flattens it again.

Typed handling for `/Additional` and optional streams lives under [`schdoc/`](../../crates/altium-format/src/schdoc/). New streams must be fully parsed before being marked consumed.

## SchLib structure

SchLib uses a root `/FileHeader`, optional `/SectionKeys`, and a storage per component. Each component has a `Data` stream and may have `Additional` plus typed pin sidecars.

The canonical sidecar names are constants in [`constants/streams.rs`](../../crates/altium-format-types/src/constants/streams.rs):

`PinFrac`, `PinDesc`, `PinMiscData`, `PinTextData`, `PinWideText`, `PinSymbolLineWidth`, `PinPackageLength`, `PinPropagationDelay`, and `PinFunctionData`.

Parsing and serialization are implemented in [`schlib.rs`](../../crates/altium-format/src/schlib.rs). Sidecar data is merged into typed pin fields; it is never retained as an opaque payload.

