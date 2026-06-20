# Container and Encoding

## Containers

`.SchLib`, `.SchDoc`, `.PcbLib`, `.PcbDoc`, and `.IntLib` use OLE Compound File Binary (CFB) containers. `.PrjPcb` is the exception: it is a UTF-8 INI-style text file.

CFB access and consumption tracking are implemented in:

- [`cfb_document.rs`](../../crates/altium-format/src/cfb_document.rs)
- [`tracked_cfb.rs`](../../crates/altium-format/src/tracked_cfb.rs)

`TrackedCfbDocument::assert_all_consumed()` is a safety boundary. A parser must read and type every encountered stream or return an error.

## Block framing

Streams that use Altium block framing contain a four-byte little-endian header followed by the payload:

```text
bits  0..23: payload length
bits 24..31: format flag (0x00 text, 0x01 binary)
```

See [`block_stream.rs`](../../crates/altium-format/src/block_stream.rs) and the named constants in [`constants/parsing.rs`](../../crates/altium-format-types/src/constants/parsing.rs).

This framing is not universal. PCB raw binary sections, prefixed parameter sections, WideStrings tables, and other typed sidecars have their own parsers. Never apply block framing based only on the fact that data is stored in CFB.

## Parameter strings

Schematic records and several PCB metadata streams use pipe-delimited `KEY=VALUE` parameters.

- Legacy text is decoded as Windows-1252.
- `%UTF8%` keys carry strict UTF-8 values.
- UTF-16LE sidecars must reject decode errors.
- Parameter keys are case-insensitive and duplicate handling is implemented by [`ParameterCollection`](../../crates/altium-format/src/param_collection.rs).
- Parsers remove typed fields and call `assert_exhausted()`; unconsumed parameters are errors.

## Coordinates

[`Coord`](../../crates/altium-format-types/src/coord.rs) stores one mil as 10,000 internal units. PCB binary records store this raw `i32` value.

Schematic parameter coordinates use an integer and optional `_FRAC` companion. Reconstruction uses a 100,000-internal-unit DXP base:

```text
raw = integer * 100_000 + fraction
```

The 100,000 value is the schematic split base, not the global units-per-mil value.

Colors use Win32 `COLORREF` byte order `0x00BBGGRR` and the domain [`Color`](../../crates/altium-format-types/src/color.rs) type.

