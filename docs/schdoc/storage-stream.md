# Storage Stream

The `Storage` stream holds embedded binary objects, primarily images referenced by
`SchImage` records (RECORD=30). This format is identical to the SchLib `/Storage` stream.

## Stream layout

```
Block 0:    Header record (flags=0x00, parameter text)
Block 1..N: Embedded objects (flags=0x01, binary with 0xD0 tag)
```

## Block 0: Header

| Key | Type | Description |
|-----|------|-------------|
| `HEADER` | string | `Icon storage` |
| `Weight` | i32 | Number of embedded object blocks that follow |

## Embedded object blocks (flags=0x01)

Each entry block payload uses the embedded object envelope format:

```
Offset  Size    Description
0x00    1       0xD0 tag (embedded object marker)
0x01    1       id_length (length of the identifier string)
0x02    N       id (ASCII string -- original file path of the embedded image)
0x02+N  4       inner header: bits[23:0]=compressed_data_length, bits[31:24]=flags
0x06+N  M       compressed data (zlib, typically starts with 0x78 0x9C)
```

The `id` field is the original Windows file path of the image (e.g.,
`D:\Saniok\Lime Micro\...\LimeMicroLogoPCB.bmp`). This matches the `FILENAME` parameter
of the corresponding `SchImage` record (RECORD=30).

The compressed data is zlib-compressed. Decompression yields the raw image data
(BMP, PNG, or other image format).

## Linking to SchImage records

`SchImage` records (RECORD=30) in the FileHeader stream reference embedded objects via
the `FILENAME` key. The `FILENAME` value must match the `id` string in the Storage stream
entry.

Example flow:
1. FileHeader stream contains: `RECORD=30|...|FileName=D:\path\logo.bmp|EmbedImage=T|...`
2. Storage stream contains an entry block with id = `D:\path\logo.bmp`
3. Decompress the entry block's data to get the raw image bytes

## Observations from real files

| File | Weight | Total size | Notes |
|------|--------|-----------|-------|
| Simple diagrams (01-03) | 2-3 | 187-269 KB | Multiple embedded images |
| Complex schematics (04, 06) | 1 | 6 KB | Single logo image |
| Complex schematics (05, 08) | 3 | 57 KB | Logo + additional images |
| 09_Misc | 4 | 195 KB | Logo + diagrams |

Files that use template graphics with embedded logos always have at least one image in
the Storage stream. The logo images are shared across all sheets in the project via the
template.
