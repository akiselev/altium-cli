# Parameters Stream

Each footprint's `<FootprintName>/Parameters` stream contains a single parameter block
with metadata about the footprint.

## Format

```
[4 bytes] u32 LE: total block length (includes string length byte + string)
[1 byte]  u8: string length (N)
[N bytes] Win1252 parameter string (pipe-delimited)
```

This is the same length-prefixed string format used by PcbLib Data stream pattern name
blocks and PcbDoc parameter sections.

## Parameter keys

| Key | Example | Required | Description |
|-----|---------|:--------:|-------------|
| `PATTERN` | `CAP0402` | Yes | Footprint name (must match storage name or SectionKeys entry) |
| `HEIGHT` | `21.6535mil` | Yes | Component height (with unit suffix) |
| `DESCRIPTION` | `Chip Capacitor, Body 1.0x0.5mm` | Yes | Human-readable description |
| `ITEMGUID` | `{6BB694B2-4D0E-4A20-BCC8-3F1719C76F09}` | No | Item GUID (may be empty) |
| `REVISIONGUID` | `{9B8FF8BD-0664-49C8-92EE-40709DC02652}` | No | Revision GUID (may be empty, NUL-terminated) |

## Examples

### Populated footprint (CAP0402 from LimeMicro)

```
|PATTERN=CAP0402|HEIGHT=21.6535mil|DESCRIPTION=Chip Capacitor, Body 1.0x0.5mm, 0402, IPC Medium Density|ITEMGUID=6BB694B2-4D0E-4A20-BCC8-3F1719C76F09|REVISIONGUID=9B8FF8BD-0664-49C8-92EE-40709DC02652\0
```

### Blank footprint (PCBComponent_1)

```
|PATTERN=PCBComponent_1|HEIGHT=0mil|DESCRIPTION=|ITEMGUID=|REVISIONGUID=\0
```

## Notes

- The `HEIGHT` value uses the `<number>mil` format — the numeric value is in mils.
- The `REVISIONGUID` value typically has a trailing NUL byte (`\0`). This appears to be
  a serialization artifact rather than meaningful data.
- GUID values may or may not have surrounding braces depending on the Altium version.
- For blank/new footprints, `HEIGHT`, `DESCRIPTION`, `ITEMGUID`, and `REVISIONGUID`
  may all be empty strings.

## Relationship to ComponentParamsTOC

The `/Library/ComponentParamsTOC/Data` stream provides a summary view of these same
parameters for all footprints, allowing Altium to display the footprint list without
reading each individual Parameters stream. The TOC contains `Name`, `Pad Count`,
`Height`, and `Description` for each footprint.
