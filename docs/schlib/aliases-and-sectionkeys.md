# Aliases and SectionKeys

## SectionKeys stream

CFB storage names are limited to 31 characters. When any component name exceeds this
limit, the `/SectionKeys` stream provides the full-name-to-short-key mapping for all such
components.

### Format

The `/SectionKeys` stream is a single parameter text block (flags=0x00):

```
|RECORD=0|KeyCount=N|LibRef0=<full_name>|SectionKey0=<short_key>|LibRef1=...|SectionKey1=...|
```

Indices are 0-based. `KeyCount` gives the number of entries.

### Name truncation rules

1. Characters in the set `` /\:*?"<>|! `` are replaced with `_`.
2. If the result is <= 31 characters, it is used directly as the CFB storage name and no
   `SectionKeys` entry is created.
3. If the result exceeds 31 characters, it is truncated to 31 characters.
4. If the truncated name collides with an existing key, a numeric suffix is appended and
   the name is re-truncated to fit within 31 characters.
5. All components whose names required truncation appear in `SectionKeys`.

### Lookup during load

When loading a component by name:
1. Apply the character-replacement rules to the requested name.
2. Check `/SectionKeys` for an entry with `LibRef{N}` matching the full (pre-truncation)
   name. If found, use the corresponding `SectionKey{N}` as the CFB storage key.
3. If no `SectionKeys` entry exists (or the stream is absent), use the character-replaced
   name directly as the CFB storage key.

## Alias system

Components can have aliases - alternative names that resolve to the same component
definition. Aliases are tracked in two places simultaneously:

1. **FileHeader component index**: The `AliasCount{N}` and `Comp{N}Alias{M}` keys list
   every alias for every component.
2. **Redirection streams**: Each alias has its own CFB sub-storage with a `Redirection`
   stream.

### Redirection stream format

An alias sub-storage is located at `/<AliasKey>/` where `<AliasKey>` is the CFB storage
key derived from the alias name (same rules as component keys). It contains a single
stream named `Redirection`, which is a single parameter text block (flags=0x00):

```
|RECORD=0|SectionName=<canonical_component_name>|
```

`SectionName` is the **full canonical component name** (not the CFB key). This is the
name to look up in the FileHeader component index and then resolve to a CFB key for
loading.

### Alias resolution during single-component load

When loading by name (e.g., via a `LibraryReference` lookup):

1. Compute the CFB key for the requested name using the SectionKeys lookup.
2. Attempt to open `/<key>/Redirection` stream.
   - If it exists: read `SectionName`, resolve that name to its CFB key, and load that
     component's `Data` stream instead.
3. If no `Redirection` stream exists but `/<key>/Data` exists: it is a canonical
   component; load it directly.
4. If neither stream exists: search the FileHeader component index for an `Alias` entry
   matching the requested name and resolve from there.

### Alias resolution during full library load

During a full library load, the `FindFirstStream("Data")` enumeration visits all sub-
storages. Sub-storages that contain a `Redirection` stream instead of a `Data` stream
are aliases. They are registered in the component index but their data is not re-parsed
(the canonical component's `Data` stream is used).
