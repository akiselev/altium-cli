# altium-cli

Rust CLI and crates to query and modify Altium files.

As of 2/24/26 this crate is in the middle of an unreleased massive rewrite for 0.2.0

## WARNING

These are complex file formats full of domain knowledge and decades of legacy cruft. I do my best to validate and test the CLI against thousands of real world files available on Github, but at the end of the day this software was built on millions of lines of vibe-reverse engineered Delphi and C# code so there WILL be subtle bugs.

This software is provided as-is without warranty. YOU area responsible for anything you send to the fab. Make sure your fab runs their own design rule checks.

## KNOWN LIMITATIONS

* Although `altium-cli` supports older file formats when reading, all mutations RESAVE THE FILE IN THE LATEST FORMAT (Altium/DXP 26 as of the time of this writing). If you are stuck on an older version of Altium, you may have problems opening the files afterwards.
* Our implementation is more strict than Altium's because of ambiguity in the reverse engineered implementation.
* This is mostly developed as a tool for agents to use, so expect lots of breaking changes.
* Only supports CFB format Altium files. Legacy ASCII PcbDocs are not supported.


## Workspace crates

- `altium-format-types`: domain types, enums, constants
- `altium-format-derive`: proc-macro derives
- `altium-format`: core parsers/serializers for Altium formats
- `autopcb-spec`: spec DSL compiler, executor, and reconciler
- `altium-cli`: command-line interface

## Current CLI commands

- `new {schdoc,schlib,pcblib,prjpcb} <output>` — create blank Altium documents
- `validate <path>` — validate Altium files (.SchLib, .PcbLib, .SchDoc, .PcbDoc, .PrjPcb, .IntLib)
- `save-as <input> <output>` — roundtrip parse and re-save
- `render <path> [-o dir] [--format svg|png]` — render SchLib/PcbLib/SchDoc to SVG/PNG
- `plan <spec>` — preview changes (ECO dry run)
- `apply <spec>` — apply spec file to create/update Altium document
- `dump <document>` — reverse-generate spec from Altium file
- `info <path>` — document summary
- `query <path> "<AQL>"` — query with Altium Query Language
- `inspect <pcbdoc> {summary,components,nets,board-outline,rules}`
- `placement solve <spec> --target <pcbdoc>` — component placement solver
- `cfb ls|dump|blocks|diff|cat ...` — CFB/OLE container inspection

### IntLib support

IntLib (Integrated Library) files can be parsed and dumped to extract embedded
schematic symbols and PCB footprints:

```bash
altium-cli validate vendor.IntLib    # reports SchLib/PcbLib counts
altium-cli dump vendor.IntLib        # produces vendor.sym + vendor.sym
```

## Roadmap

- Gerber output
- Rendering parts, footprints, and documents to image/pdf


## License

Apache-2.0
