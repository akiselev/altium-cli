# Rendering

Rendering uses the `AltiumCanvas` abstraction in [`render/`](../crates/altium-format/src/render/) with SVG and PNG backends in `altium-format-render-svg` and `altium-format-render-png`.

The CLI supports:

```bash
altium render <file> --output-dir <dir> --format svg|png [--name <entity>] [--scale <pixels-per-mil>]
```

Current document support:

- SchLib components: SVG and PNG
- SchDoc sheets: SVG and PNG
- PcbLib footprints: SVG and PNG
- PcbDoc, PrjPcb, and IntLib: not rendered

The current code implements schematic width tables, dash styles, junction sizes, Tahoma fallback, and SVG transform groups. Do not use historical rendering plans as a status source; use the backend implementations and [`STATUS.md`](../STATUS.md).

