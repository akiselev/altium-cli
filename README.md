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
- `altium-format-ops`: high-level operations and `.ops` execution
- `altium-cli`: command-line interface

## Current CLI commands

- `new schdoc <output>`
- `new schlib <output>`
- `validate <path>`
- `save-as <input> <output>`
- `get version <path>`
- `ops apply <path> --spec-file <file.ops> [--output <path>] [--dry-run] [--report-json]`
- `cfb ls|dump|blocks|diff|cat ...` (CFB/OLE inspection/debugging)

## Roadmap

- Gerber output
- Rendering parts, footprints, and documents to image/pdf


## License

Apache-2.0
