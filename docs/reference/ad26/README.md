# AD26 Source Reference Snapshots

These files summarize decompiled Altium Designer 26 .NET and Delphi sources. They were imported on 2026-02-21 and 2026-02-22 and are retained as reverse-engineering evidence.

They are not current `altium-cli` implementation documentation. File paths, API coverage, and implementation-status remarks inside them may be stale. Validate any claim against current Rust code plus `AD26-dotnet/` or the relevant Delphi DLL before changing a parser.

- `file-format-constants.md`: schematic format constants from AD26 source
- `sch-dotnet-model.md` / `pcb-dotnet-model.md`: managed model interfaces
- `sch-api-functions.md` / `pcb-api-functions.md`: Delphi API inventories
- `dotnet-delphi.md`: managed/native interop investigation
- `altium-types.md` / `altium-constants.md`: legacy DXP type and constant catalogs
- `gerber.md`: Gerber exporter/source investigation

