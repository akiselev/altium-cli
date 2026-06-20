# PCB Files

## Object types and binary primitives

The complete PCB object discriminant catalog is [`PcbObjectId`](../../crates/altium-format-types/src/pcb.rs). The implemented PcbLib primitive variants and their typed parsers live under [`pcblib/`](../../crates/altium-format/src/pcblib/). PcbDoc reuses those layouts where the formats agree and has document-specific parsers under [`pcbdoc/`](../../crates/altium-format/src/pcbdoc/).

The shared 13-byte primitive header is parsed by [`parse_common_header`](../../crates/altium-format/src/pcblib/primitives/common.rs):

```text
offset  size  field
0       1     layer: V6Layer
1       2     flags: PcbFlags
3       2     net_index
5       2     polygon_index
7       2     component_index
9       2     coordinate_index
11      2     dimension_index
```

All index fields are `u16`; `0xFFFF` means no association.

## PcbLib structure

PcbLib stores library-wide data under `/Library` and one CFB storage per footprint. A footprint `Data` stream contains its pattern-name block followed by mixed typed primitives. `/SectionKeys` maps names that exceed CFB storage-name limits.

PcbLib footprint `WideStrings` is one length-prefixed parameter string containing `ENCODEDTEXT{N}` values. Each value is a comma-separated sequence of UTF-16 code units. The active parser and serializer are [`pcblib/wide_strings.rs`](../../crates/altium-format/src/pcblib/wide_strings.rs).

## PcbDoc structure

PcbDoc uses root-level sections such as `Board6`, `Pads6`, `Tracks6`, `Nets6`, and `Rules6`, normally with `Header` and `Data` streams. Section dispatch and full-consumption checks are implemented in [`pcbdoc/mod.rs`](../../crates/altium-format/src/pcbdoc/mod.rs).

PcbDoc `WideStrings6/Data` uses indexed UTF-16LE records:

```text
[u32 index][u32 byte_length][UTF-16LE payload]
```

The empty-string sentinel is `byte_length == 2` with no payload bytes. The active parser is [`parse_wide_strings6_records`](../../crates/altium-format/src/pcbdoc/records.rs). Do not substitute the unrelated type-tagged string codec in `wide_strings_tlv.rs` for this section.

## Safety boundary

Version-dependent tails are not preserved as `trailing_bytes`. A parser must type every byte for the accepted layout and call `assert_exhausted()`, or reject the record with stream, record, and offset context.

