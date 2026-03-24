# PrjPcb Project File Documentation

Research and design documents for `.PrjPcb` (Altium PCB project file) support.

## Documents

| Document | Purpose |
|----------|---------|
| [format.md](format.md) | Complete file format reference — sections, keys, enums, parsing mechanics |
| [high-level-api.md](high-level-api.md) | High-level API design — types, methods, read/write paths, executor/reconciler patterns |
| [spec-lang-design.md](spec-lang-design.md) | Spec language extension design — `.proj` syntax, AST/model changes, implementation phases |

## Quick Facts

- **Not a CFB container** — plain-text INI format (the only Altium file type that isn't CFB)
- **Encoding:** UTF-8 with BOM
- **Parsing:** Line-by-line, split on `[Section]` headers and `Key=Value` on first `=`
- **Test fixture:** `data/BlankProject.PrjPcb` (1090 lines, 10 output groups, blank ERC)
- **C# reader:** `PrjPcbReader` → `PrjPcbContent` (read-only; writing is Delphi-side)
- **Current Rust status:** Stub only (`project.rs` — empty struct, `open()` reads but doesn't parse)

## C# Source Files

| File | Content |
|------|---------|
| `AD26-dotnet/Altium.Sch.Data.Project/…/PrjPcbReader.cs` | Reader entry point |
| `AD26-dotnet/Altium.Sch.Data.Project/…/PrjPcbContent.cs` | Parser + accessor methods |
| `AD26-dotnet/Altium.Sch.Data.Project/…/PrjPcbConsts.cs` | Key name constants |
| `AD26-dotnet/Altium.Sch.Base/…/PrjContentBase.cs` | Base class: line parsing, state machine |
| `AD26-dotnet/Altium.Sch.Data.Project/…/ProjectOptions.cs` | Immutable output data class |
