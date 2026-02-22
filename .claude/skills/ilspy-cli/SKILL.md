---
name: ilspy-cli
description: >
    Use ilspy-cli for .NET assembly decompilation and analysis. Produces clean C# source from .NET IL bytecode.
    Activate when the user requests:
    - .NET assembly decompilation
    - .NET type or method inspection
    - Detecting whether a binary is .NET or native
    - Searching decompiled .NET source code
    - Assembly metadata inspection
    Prefer ilspy-cli over ghidra-cli for .NET binaries (ghidra produces poor output for .NET IL).
---

# ilspy-cli Agent Reference

Rust CLI for .NET decompilation using ILSpy (ICSharpCode.Decompiler). Binary name: `ilspy`.

## Architecture

```
Rust CLI (clap) ──netcorehost FFI──► C# Bridge DLL (IlSpyBridge.dll) ──► ICSharpCode.Decompiler
```

- **In-process**: .NET runtime hosted inside the Rust binary via `netcorehost` crate
- **No server/daemon**: each command loads the assembly, runs, exits
- **Data exchange**: JSON strings over FFI function pointers with `[UnmanagedCallersOnly]`
- **Key differentiator**: single-method decompilation (`--type T --method M`), which `ilspycmd` cannot do

## Requirements

- .NET 8 runtime (loaded by netcorehost at runtime)
- .NET 8 SDK (build time only, for `dotnet publish` of the C# bridge)

## When to Use ilspy-cli vs ghidra-cli

| Binary type | Tool | Reason |
|-------------|------|--------|
| .NET (.dll/.exe with CLR header) | `ilspy` | Clean C# decompilation |
| Native (C/C++/Delphi/Go) | `ghidra` | Assembly-level analysis |
| Unknown | `ilspy detect` first | Classifies .NET vs native |

Use `ilspy detect` to triage before choosing a tool.

## Global Flags

| Flag | Effect |
|------|--------|
| `--json` | Minified JSON output |
| `--pretty` | Pretty-printed JSON |
| `--compact` | One line per item |
| `-v` / `--verbose` | Verbose output |

**Format auto-detection**: TTY → table; non-TTY → compact.

## Command Reference

### Detect (.NET vs Native)

Classify binaries without loading .NET runtime (pure Rust PE header parsing).

```bash
ilspy detect FILE                         # single file
ilspy detect DIRECTORY                    # scan directory (top-level)
ilspy detect DIRECTORY --recursive        # scan recursively
ilspy detect DIRECTORY --dotnet-only      # show only .NET assemblies
ilspy detect DIRECTORY --native-only      # show only native binaries
```

**Output fields**: path, isDotnet, framework, recommendedTool

Framework detection includes: .NET Core/5+/6+/7+/8+, .NET Framework, .NET Standard, Native (Delphi), Native (Qt/C++), Native (MFC/C++).

The `recommendedTool` field returns `"ilspy"` for .NET or `"ghidra"` for native.

### List Types

```bash
ilspy list types ASSEMBLY                           # all types
ilspy list types ASSEMBLY --filter Controller        # substring filter (case-insensitive)
ilspy list types ASSEMBLY --kind class               # filter by kind
ilspy list types ASSEMBLY --kind enum
ilspy list types ASSEMBLY --filter Crypto --kind class
```

**Kind values**: `class`, `interface`, `struct`, `enum`, `delegate`

**Output fields**: fullName, ns, name, kind, methodCount, propertyCount, fieldCount, isPublic

### List Methods

```bash
ilspy list methods ASSEMBLY                                  # all methods
ilspy list methods ASSEMBLY --type MyNamespace.MyClass        # methods of one type
ilspy list methods ASSEMBLY --type MyNamespace.MyClass --filter DoWork  # filter by name
```

**Output fields**: typeName, name, returnType, parameters (name + type), accessibility, isStatic, isVirtual, isAbstract

### Decompile

```bash
ilspy decompile ASSEMBLY                                    # full assembly → C# source
ilspy decompile ASSEMBLY --type MyNamespace.MyClass         # single type
ilspy decompile ASSEMBLY --type MyNamespace.MyClass --method DoWork  # single method!
```

**Output**: C# source code (default). With `--json`/`--pretty`: `{ "source": "...", "typeName": "...", "methodName": "...", "returnType": "..." }`

`--method` requires `--type` (must specify the containing type).

**Default format override**: decompile always outputs raw source unless `--json` or `--pretty` is specified.

### Search Decompiled Source

```bash
ilspy search ASSEMBLY "ConnectionString"
ilspy search ASSEMBLY "password|secret|key"
ilspy search ASSEMBLY "HttpClient\.Post"
```

Pattern is a **regex** (case-insensitive, multiline). The tool decompiles each type and runs the regex against the C# source.

**Output fields per match**: typeName, matchCount, matches[].line, matches[].matched, matches[].context (surrounding lines)

### Assembly Info

```bash
ilspy info ASSEMBLY
```

**Output fields**: name, typeCount, targetFramework, references[].name, references[].version

### Doctor (Health Check)

```bash
ilspy doctor
```

Checks: .NET runtime availability, bridge DLL presence, bridge loading.

**Environment variable**: `ILSPY_BRIDGE_DIR` overrides bridge DLL search path.

## Agent Best Practices

### 1. Triage First

Always detect before decompiling an unknown binary:

```bash
ilspy detect ./target.dll --json
# Check recommendedTool field
```

### 2. Count-First for Large Assemblies

```bash
# Check type count
ilspy info MyLib.dll --json
# If typeCount is large, filter:
ilspy list types MyLib.dll --filter Controller --json
```

### 3. Targeted Decompilation

Decompile specific types/methods instead of entire assemblies:

```bash
# GOOD: specific type
ilspy decompile MyLib.dll --type MyApp.Services.AuthService

# GOOD: specific method
ilspy decompile MyLib.dll --type MyApp.Services.AuthService --method ValidateToken

# AVOID for large assemblies: full decompile
ilspy decompile MyLib.dll
```

### 4. Search Before Decompile

Find relevant types first, then decompile:

```bash
# Find types containing crypto code
ilspy search MyLib.dll "AES|Rijndael|SHA256" --json

# Decompile the matching type
ilspy decompile MyLib.dll --type MyApp.Security.CryptoHelper
```

### 5. Use Compact/JSON for Parsing

```bash
ilspy list types MyLib.dll --json           # minified JSON for piping
ilspy list types MyLib.dll --compact        # one line per type
ilspy decompile MyLib.dll --type T --json   # source wrapped in JSON
```

## Analysis Workflow

```bash
# 1. Detect binary type
ilspy detect ./target.dll

# 2. Get overview
ilspy info ./target.dll
ilspy list types ./target.dll --json

# 3. Find interesting types
ilspy list types ./target.dll --filter Service --kind class
ilspy list types ./target.dll --filter Crypto

# 4. Inspect methods
ilspy list methods ./target.dll --type MyApp.Services.AuthService

# 5. Decompile specific method
ilspy decompile ./target.dll --type MyApp.Services.AuthService --method ValidateToken

# 6. Search for patterns
ilspy search ./target.dll "password|credential|secret"
ilspy search ./target.dll "HttpClient|WebRequest|Socket"
ilspy search ./target.dll "Process\.Start|Shell|Exec"
```

## Directory Scanning Workflow

```bash
# Triage a directory of binaries
ilspy detect "C:\Program Files\MyApp" --recursive --json

# Focus on .NET assemblies only
ilspy detect "C:\Program Files\MyApp" --recursive --dotnet-only --compact
```

## Integration with ghidra-cli

ghidra-cli's `decompile` command warns when it encounters .NET IL bytecode:
> "This appears to be .NET managed code. Consider using ilspy-cli."

Workflow for mixed codebases:

```bash
# 1. Classify all binaries
ilspy detect ./binaries --recursive --json

# 2. Use ilspy for .NET
ilspy decompile ./binaries/managed.dll --type MyClass

# 3. Use ghidra for native
ghidra import ./binaries/native.exe --project analysis
ghidra decompile main --project analysis
```

## Error Recovery

| Problem | Fix |
|---------|-----|
| "dotnet not found" | Install .NET 8 SDK: https://dot.net/download |
| "Bridge DLL not found" | `cargo build` or set `ILSPY_BRIDGE_DIR` |
| "Type not found" | Use `ilspy list types ASSEMBLY --filter NAME` to find exact fullName |
| "Method not found" | Use `ilspy list methods ASSEMBLY --type T` to see available methods |
| Bridge load fails | Run `ilspy doctor` for diagnostics |
