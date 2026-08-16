# altium-cli

> **Experimental.** The 0.2.0 line is an unreleased rewrite and the file-format implementation is still being validated against real-world Altium files.

Rust CLI and crates for reading, validating, rendering, querying, and modifying Altium files.

## Warning

Altium file formats contain substantial domain knowledge and decades of legacy behavior. The implementation is validated against thousands of real-world files where possible, but much of the format behavior was reconstructed from Delphi and C# implementations. Subtle compatibility bugs should be expected.

This software is provided as-is without warranty. You are responsible for anything you send to fabrication. Make sure your fab performs its own design-rule checks.

## Known limitations

* Although `altium-cli` supports older file formats when reading, all mutations resave the file in the latest format (Altium/DXP 26 at the time of writing). Older Altium versions may not be able to reopen the result.
* The implementation is stricter than Altium in some places where reverse-engineered behavior is ambiguous.
* The public API and CLI may change substantially while the 0.2.0 rewrite is in progress.
* Only CFB-format Altium files are supported. Legacy ASCII PcbDocs are not supported.

## Workspace crates

- `altium-format-types`: domain types, enums, constants
- `altium-format-derive`: proc-macro derives
- `altium-format`: core parsers/serializers for Altium formats
- `altium-format-spec`: declarative spec language for Altium files
- `altium-cli`: command-line interface

## Current CLI commands

- `new {schdoc,schlib,pcblib,prjpcb} <output>`: create blank Altium documents
- `validate <path>`: validate Altium files (.SchLib, .PcbLib, .SchDoc, .PcbDoc, .PrjPcb, .IntLib)
- `save-as <input> <output>`: roundtrip parse and re-save
- `render <path> [-o dir] [--format svg|png]`: render SchLib/PcbLib/SchDoc to SVG/PNG
- `plan <spec>`: preview changes (ECO dry run)
- `apply <spec>`: apply a spec file to create or update an Altium document
- `dump <document>`: reverse-generate a spec from an Altium file
- `info <path>`: print a document summary
- `query <path> "<AQL>"`: query with Altium Query Language
- `cfb ls|dump|blocks|diff|cat ...`: inspect CFB/OLE containers

Spec files use explicit document extensions: `.schlib-spec`, `.pcblib-spec`,
`.schdoc-spec`, `.pcbdoc-spec`, and `.prjpcb-spec`.

### IntLib support

IntLib (Integrated Library) files can be parsed and dumped to extract embedded
schematic symbols and PCB footprints:

```bash
altium-cli validate vendor.IntLib    # reports SchLib/PcbLib counts
altium-cli dump vendor.IntLib        # produces vendor.schlib-spec + vendor.pcblib-spec
```

## Roadmap

- Gerber output
- Rendering parts, footprints, and documents to image/PDF

## License

Apache-2.0
