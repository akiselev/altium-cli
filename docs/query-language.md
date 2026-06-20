# Altium Query Language

AQL selects entities through the high-level document APIs. The implementation is `crates/altium-format-query`; the CLI entry point is:

```bash
altium query <document> '<query>' --format text|json|count [--limit N]
```

Core selectors:

```text
component                 type selector
R*                        designator prefix
C??                       fixed-width designator pattern
$LM358                    part number
@10K                      value
%VCC                      net name
#42                       record ID
U1:VCC                    component pin
*                         any entity
```

Compound queries support attribute filters, pseudo-classes, direct-child (`>`), descendant whitespace, union (`,`), and `AND`/`OR`/`NOT`.

```text
component > pin:power
component[value="10K"]
pad:smd, via
NOT component[virtual=true]
```

The authoritative grammar and selector catalogs are [`parser.rs`](../crates/altium-format-query/src/parser.rs) and [`ast.rs`](../crates/altium-format-query/src/ast.rs). Unknown types, fields, pseudo-classes, and invalid units are errors.

