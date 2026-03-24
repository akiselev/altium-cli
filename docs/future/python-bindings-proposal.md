# Python API Design (PyO3)

Design notes for a Python binding to `autopcb-spec` via PyO3, targeted at
LLM agents writing scripts to manipulate Altium files.

## Goals

- **Robust and Pythonic**: full type annotations, IDE autocomplete, LLM-friendly
- **Simple to implement**: minimal Rust glue, most complexity lives in pure Python
- **Correct by construction**: Rust validates everything; Python layer is ergonomic sugar

---

## Architecture Overview

```
+-----------------------------+
|  altium-py  (pure Python)   |  <-- pip install altium
|  - ops.py  (dataclasses)    |
|  - enums.py                 |
|  - __init__.py (re-exports) |
|  - py.typed + *.pyi stubs   |
+-------------+---------------+
              | json.dumps()
+-------------v---------------+
|  _altium_native  (PyO3)     |  <-- Compiled Rust extension
|  - Document (open/save)     |
|  - apply_json()             |
|  - ApplyReport / OpResult   |
|  - AltiumError              |
+-------------+---------------+
              |
+-------------v---------------+
|  autopcb-spec (Rust)   |
|  (unchanged)                |
+-----------------------------+
```

The Rust crate (`_altium_native`) exposes ~5-6 classes. The Python package
(`altium-py`) provides the ergonomic typed layer on top.

### Why a JSON bridge?

The ops layer already has `parse_apply_spec_json(data) -> Vec<HighOp>` and
`apply_ops_source_*(doc, source) -> ApplyReport`. Both bypass the need to expose
individual Rust structs to Python. All `HighOp` variants and their nested types
derive `Serialize`/`Deserialize`, so JSON is the natural FFI boundary.

---

## Rust Layer (~200 LOC)

Minimal PyO3 surface: one `Document` class that wraps the four document types,
plus result types.

```rust
#[pyclass]
struct Document {
    inner: DocumentInner,  // enum { SchDoc(SchDoc), SchLib(SchLib), ... }
}

#[pymethods]
impl Document {
    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> { ... }

    #[staticmethod]
    fn new_blank(doc_type: &str) -> PyResult<Self> { ... }

    fn save(&self, path: &str) -> PyResult<()> { ... }
    fn validate(&self) -> PyResult<()> { ... }
    fn version(&self) -> PyResult<VersionInfo> { ... }
    fn doc_type(&self) -> &str { ... }

    fn apply_json(&mut self, json: &str) -> PyResult<ApplyReport> {
        let ops = parse_apply_spec_json(json)?;
        match &mut self.inner {
            DocumentInner::SchLib(lib) => Ok(apply_schlib(lib, &ops)?.into()),
            DocumentInner::SchDoc(doc) => Ok(apply_schdoc(doc, &ops)?.into()),
            DocumentInner::PcbDoc(doc) => Ok(apply_pcbdoc(doc, &ops)?.into()),
            DocumentInner::PcbLib(lib) => Ok(apply_pcblib(lib, &ops)?.into()),
        }
    }

    fn apply_ops(&mut self, source: &str) -> PyResult<ApplyReport> { ... }
}
```

Result types exposed as `#[pyclass]`:

```rust
#[pyclass]
struct ApplyReport {
    #[pyo3(get)] high_op_count: usize,
    #[pyo3(get)] composed_op_count: usize,
    #[pyo3(get)] low_op_count: usize,
    #[pyo3(get)] results: HashMap<String, OpResult>,
}

#[pyclass]
struct OpResult {
    #[pyo3(get)] opid: String,
    #[pyo3(get)] kind: String,
    #[pyo3(get)] ref_: Option<EntityRef>,
    #[pyo3(get)] refs: Vec<EntityRef>,
    #[pyo3(get)] fields: PyObject,      // dict[str, Any]
    #[pyo3(get)] warnings: Vec<String>,
}

#[pyclass]
struct EntityRef {
    #[pyo3(get)] domain: String,
    #[pyo3(get)] entity_type: String,
    #[pyo3(get)] id: String,
    #[pyo3(get)] display_path: String,
}

#[pyclass]
struct VersionInfo {
    #[pyo3(get)] header: String,
    #[pyo3(get)] minor_version: i32,
    #[pyo3(get)] file_version_info: Option<String>,
}
```

---

## Python Layer (~300-400 LOC)

### Design Decision Summary

| Axis             | Choice                              | Rationale                                            |
| ---------------- | ----------------------------------- | ---------------------------------------------------- |
| Representation   | `dataclasses`                       | Zero deps, LLM-friendly, type-checkable              |
| Enums            | Python `Enum` classes               | Autocomplete; also accept raw str/int                 |
| Coordinates      | Raw `int` (mils), `_mils` suffix    | Matches JSON spec, zero conversion                   |
| Op union type    | Explicit `Op` type alias            | Full type checking                                   |
| Cross-references | `str` opid + accept Op objects      | Simple default, Pythonic sugar                        |
| Serialization    | Generic `to_spec()` + `@op` decorator | ~30 LOC, zero per-class boilerplate                |

Each decision is discussed in detail below.

---

## Axis 1: How Ops Are Represented in Python

### Option 1A: Dataclasses (stdlib) -- RECOMMENDED

```python
from __future__ import annotations
from dataclasses import dataclass, field

@dataclass
class AddComponent:
    lib_reference: str
    designator: str = ""
    value: str = ""
    description: str = ""
    pins: list[Pin] = field(default_factory=list)
    footprint: Footprint | None = None
    opid: str | None = None

@dataclass
class Pin:
    designator: str
    name: str = ""
    electrical: PinElectrical = PinElectrical.PASSIVE
    at: tuple[int, int] = (0, 0)
    length_mils: int = 200
    rotation: Rotation = Rotation.RIGHT
```

**Pros:**
- Zero dependencies, stdlib only
- `asdict()` gives JSON-ready dicts for free
- Type checkers (mypy/pyright) understand them natively
- LLMs know dataclasses extremely well
- IDE autocomplete works perfectly
- Simple to maintain

**Cons:**
- No runtime validation (you can pass `electrical="garbage"` -- won't fail until Rust)
- `asdict()` doesn't handle enums -- needs a custom serializer
- No fluent/builder chaining

### Option 1B: Pydantic models

```python
from pydantic import BaseModel, Field
from typing import Annotated

class Pin(BaseModel):
    designator: str
    name: str = ""
    electrical: PinElectrical = PinElectrical.PASSIVE
    at: tuple[int, int] = (0, 0)
    length_mils: Annotated[int, Field(ge=0)] = 200
    rotation: Rotation = Rotation.RIGHT

class AddComponent(BaseModel):
    lib_reference: str
    designator: str = ""
    value: str = ""
    pins: list[Pin] = []
    footprint: Footprint | None = None
    opid: str | None = None

    def model_dump_op(self) -> dict:
        d = self.model_dump(exclude_none=True)
        d["op"] = "add_component"
        return d
```

**Pros:**
- Runtime validation (wrong types, out-of-range values caught immediately in Python)
- `.model_dump()` handles enums, nested models, serialization correctly
- `.model_json_schema()` can auto-generate JSON Schema for LLM tool-use
- Immutability by default (frozen models)
- LLMs know pydantic well (it's the standard for LLM tool definitions)

**Cons:**
- Adds a dependency (`pydantic>=2`)
- Heavier than dataclasses
- Pydantic v2 vs v1 API is a common pain point
- Double validation (pydantic validates, then Rust validates again)

### Option 1C: TypedDict (no classes at all)

```python
from typing import TypedDict, NotRequired, Literal

class Pin(TypedDict):
    designator: str
    name: NotRequired[str]
    electrical: NotRequired[str]
    at: NotRequired[tuple[int, int]]
    length_mils: NotRequired[int]
    rotation: NotRequired[int]

class AddComponentOp(TypedDict):
    op: Literal["add_component"]
    lib_reference: str
    designator: NotRequired[str]
    value: NotRequired[str]
    pins: NotRequired[list[Pin]]
    opid: NotRequired[str]
```

```python
# Usage -- just dicts, but type-checked
ops: list[Op] = [
    {"op": "add_component", "lib_reference": "Resistor", "designator": "R1",
     "pins": [{"designator": "1", "name": "A", "electrical": "Passive"}]},
]
report = doc.apply_json(json.dumps(ops))
```

**Pros:**
- Zero overhead -- they're just dicts
- Type checkers still validate structure
- Serialization is trivial (`json.dumps` directly)
- Closest to what the Rust serde layer expects

**Cons:**
- No runtime validation at all
- No IDE autocomplete on construction (just on access)
- Ugly syntax -- no `AddComponent(...)` constructor
- Enums are raw strings, no autocomplete
- LLMs often produce invalid dicts (missing required keys, wrong nesting)

### Option 1D: attrs

```python
import attrs

@attrs.define
class Pin:
    designator: str
    name: str = ""
    electrical: PinElectrical = PinElectrical.PASSIVE
    at: tuple[int, int] = (0, 0)
    length_mils: int = attrs.field(default=200, validator=attrs.validators.ge(0))
    rotation: Rotation = Rotation.RIGHT
```

**Pros:** Similar to dataclasses but with built-in validators, slots by default,
smarter `attrs.asdict()`.

**Cons:** Another dependency, less known to LLMs than dataclasses or pydantic.

### Verdict

**Dataclasses** for the initial version:
- Zero deps keeps the package light
- You can always add a pydantic compatibility layer later
- LLMs generate dataclass code very reliably
- The Rust layer already validates -- double validation is waste
- A small custom `to_json()` helper (~10 lines) handles enum serialization

---

## Axis 2: How Enums Are Exposed

~12 enums are used in op fields.

### Option 2A: Python Enum classes mirroring Rust -- RECOMMENDED

```python
from enum import Enum

class PinElectrical(Enum):
    INPUT = "Input"
    IO = "IO"
    OUTPUT = "Output"
    OPEN_COLLECTOR = "OpenCollector"
    PASSIVE = "Passive"
    HI_Z = "HiZ"
    OPEN_EMITTER = "OpenEmitter"
    POWER = "Power"

class Rotation(Enum):
    RIGHT = 0
    UP = 90
    LEFT = 180
    DOWN = 270

class LineStyle(Enum):
    SOLID = 0
    DASHED = 1
    DOTTED = 2
    DASH_DOTTED = 3

class TextJustification(Enum):
    BOTTOM_LEFT = 0
    BOTTOM_CENTER = 1
    BOTTOM_RIGHT = 2
    CENTER_LEFT = 3
    CENTER = 4
    CENTER_RIGHT = 5
    TOP_LEFT = 6
    TOP_CENTER = 7
    TOP_RIGHT = 8

class ComponentKind(Enum):
    STANDARD = 0
    MECHANICAL = 1
    GRAPHICAL = 2
    NET_TIE_BOM = 3
    NET_TIE_NO_BOM = 4
    STANDARD_NO_BOM = 5
    JUMPER = 6

class PenWidth(Enum):
    ZERO = 0
    SMALL = 1
    MEDIUM = 2
    LARGE = 3

class LineShape(Enum):
    NONE = 0
    ARROW = 1
    SOLID_ARROW = 2
    TAIL = 3
    SOLID_TAIL = 4
    CIRCLE = 5
    SQUARE = 6

class HorizontalAlign(Enum):
    CENTER = 0
    LEFT = 1
    RIGHT = 2
```

**Pros:** IDE autocomplete, typo prevention, self-documenting.
**Cons:** Must keep in sync with Rust.

### Option 2B: String literals with Literal type

```python
PinElectrical = Literal[
    "Input", "IO", "Output", "OpenCollector",
    "Passive", "HiZ", "OpenEmitter", "Power"
]
```

**Pros:** Zero overhead, mypy/pyright check valid values.
**Cons:** No autocomplete on values, easy to drift.

### Option 2C: Module-level constants

```python
# altium/electrical.py
INPUT = "Input"
IO = "IO"
OUTPUT = "Output"
PASSIVE = "Passive"
```

**Pros:** Simple, importable.
**Cons:** No grouping, no type narrowing.

### Verdict

**Enum classes** for commonly used ones (`PinElectrical`, `Rotation`, `LineStyle`,
`TextJustification`). They give the best LLM experience -- an agent can write
`PinElectrical.PASSIVE` and get it right. But also accept the raw string/int
value so users aren't forced to import enums for one-off usage:

```python
# Both work:
Pin(designator="1", electrical=PinElectrical.PASSIVE)
Pin(designator="1", electrical="Passive")  # also accepted
```

This is easy with a custom serializer that calls
`val.value if isinstance(val, Enum) else val`.

---

## Axis 3: Coordinate Representation

The ops layer uses mils as integers. Internal representation is `Coord`
(10,000 units/mil). The question is what Python sees.

### Option 3A: Raw mils as `int` -- RECOMMENDED

```python
Pin(designator="1", at=(100, 200), length_mils=300)
AddTrack(start=(0, 0), end=(500, 500), width_mils=10)
```

**Pros:** Matches the JSON spec exactly. Simple. No conversion.
**Cons:** Field names must include `_mils` to avoid ambiguity.

### Option 3B: Named coordinate type

```python
class Mils(int):
    """Coordinate in mils (1 mil = 0.001 inch)."""
    pass

class Mm(float):
    """Coordinate in millimeters."""
    def to_mils(self) -> int:
        return round(self * 39.3701)

Pin(designator="1", at=(Mils(100), Mils(200)))
Pin(designator="1", at=(Mm(2.54), Mm(5.08)))  # auto-converts
```

**Pros:** Self-documenting, supports mm input.
**Cons:** Overhead, LLMs might not use the wrapper types.

### Option 3C: Just int, document as mils

```python
Pin(designator="1", at=(100, 200), length=300)
# Docstring: "All coordinates are in mils (1 mil = 0.001 inch)"
```

**Pros:** Minimal.
**Cons:** Ambiguous without reading docs.

### Verdict

**3A (raw ints, suffix `_mils` where ambiguous)**. It matches what the JSON layer
expects, which means zero conversion code. The `_mils` suffix is already in the
Rust op struct field names. Add a module-level utility for users who think in mm:

```python
def mm(val: float) -> int:
    """Convert millimeters to mils."""
    return round(val / 0.0254)
```

---

## Axis 4: The Op Union Type and apply()

### Option 4A: Explicit Union type -- RECOMMENDED (combined with 4C)

```python
Op = (
    AddComponent | AddPin | AddParameter | AddAlias | RemoveAlias |
    RemoveComponent | EditComponent | EditRecord | RemoveRecords |
    Query | QueryComponents | QueryPins | QueryRecords |
    AddLine | AddRectangle | AddArc | AddEllipticalArc | AddEllipse |
    AddPolyline | AddPolygon | AddBezier | AddPie | AddRoundRectangle |
    AddLabel | AddTextFrame | AddImage |
    AddTrack | AddVia | AddFootprint
)

def apply(doc: Document, ops: Sequence[Op]) -> ApplyReport: ...
```

**Pros:** Full type checking -- pyright catches if you pass a non-op object.
**Cons:** Giant union, must be updated for every new op.

### Option 4B: Protocol-based

```python
from typing import Protocol, runtime_checkable

@runtime_checkable
class OpLike(Protocol):
    def to_spec(self) -> dict[str, Any]: ...

def apply(doc: Document, ops: Sequence[OpLike]) -> ApplyReport: ...
```

**Pros:** Open for extension, any class with `to_spec()` works.
**Cons:** Less precise type checking, no exhaustive autocomplete for "what ops exist".

### Option 4C: Method on Document

```python
class Document:
    def apply(self, *ops: Op) -> ApplyReport: ...
    # or
    def apply(self, ops: Sequence[Op]) -> ApplyReport: ...
```

**Pros:** Discoverable.
**Cons:** Still needs the union type.

### Verdict

**4A + 4C combined.** Define the explicit `Op` union type for type checking. Use
`doc.apply(ops)` as the primary API. Also provide `doc.apply_json()` and
`doc.apply_ops()` as escape hatches for raw JSON/DSL strings.

---

## Axis 5: Cross-References Between Ops (RefExpr)

Ops support `component_ref` / `footprint_ref` to reference previous op results.

### Option 5A: String-based opid references

```python
comp = AddComponent(opid="r1", lib_reference="Resistor", ...)
pin = AddPin(component_ref="r1", designator="1", ...)
```

Where `component_ref="r1"` is shorthand for `RefExpr(root=OpId("r1"))`.

**Pros:** Dead simple. LLMs handle strings trivially.
**Cons:** No type checking that "r1" exists. Accessing nested refs
(`$r1.pins[0]`) needs a mini-DSL in a string.

### Option 5B: OpRef helper class

```python
@dataclass
class OpRef:
    op_id: str
    _steps: list[str | int] = field(default_factory=list)

    def __getattr__(self, name: str) -> OpRef:
        new = OpRef(self.op_id, [*self._steps, name])
        return new

    def __getitem__(self, idx: int) -> OpRef:
        return OpRef(self.op_id, [*self._steps, idx])

    def to_spec(self) -> dict: ...

# Usage:
r1 = OpRef("r1")
AddPin(component_ref=r1, ...)           # ref to component
AddLine(component_ref=r1.pins[0], ...)  # ref to first pin
```

**Pros:** Fluent, type-safe-ish, looks like Python attribute access.
**Cons:** `__getattr__` magic is fragile, mypy can't type-check the chain.

### Option 5C: Explicit ref builders

```python
def op_ref(op_id: str) -> RefExpr: ...
def last() -> RefExpr: ...

class RefExpr:
    def member(self, name: str) -> RefExpr: ...
    def index(self, idx: int) -> RefExpr: ...
```

```python
AddPin(component_ref=op_ref("r1"), designator="1")
AddLine(component_ref=op_ref("r1").member("pins").index(0))
```

**Pros:** Explicit, no magic, type-checkable.
**Cons:** Verbose. `ref()` shadows a builtin (use `op_ref()` instead).

### Option 5D: Accept the op object itself

```python
comp = AddComponent(opid="r1", lib_reference="Resistor", ...)
pin = AddPin(component_ref=comp, designator="1")  # pass the object
```

Where `apply()` resolves `comp` -> its `opid` during serialization.

**Pros:** Most Pythonic, feels natural.
**Cons:** Doesn't work for `$last` or `$self`. Needs special handling in
`to_spec()`. Can't reference ops from a previous `apply()` call.

### Verdict

**5A (string) as the default, with 5D (object) as sugar.** The serializer checks:
if `component_ref` is a string, treat it as an opid. If it's an op dataclass,
extract its `opid`. This covers 95% of use cases with zero ceremony:

```python
comp = AddComponent(opid="r1", lib_reference="Resistor")
pin = AddPin(component_ref=comp, designator="1")       # object ref
line = AddLine(component_ref="r1", from_=(0,0), to=(100,100))  # string ref
```

For the rare case of `$last` or nested paths, accept a `RefExpr` too:

```python
component_ref: str | Op | RefExpr | None = None
```

---

## Axis 6: Serialization -- How Dataclasses Become JSON

### Option 6A: `dataclasses.asdict()` + custom encoder

```python
import json
from dataclasses import asdict

class OpEncoder(json.JSONEncoder):
    def default(self, obj):
        if isinstance(obj, Enum):
            return obj.value
        if hasattr(obj, 'opid'):  # it's an Op, resolve to ref
            return {"op_id": obj.opid}
        return super().default(obj)

def ops_to_json(ops: Sequence[Op]) -> str:
    specs = []
    for op in ops:
        d = asdict(op)
        d["op"] = _OP_TAG[type(op)]  # "add_component", etc.
        d = {k: v for k, v in d.items() if v is not None}  # strip None
        specs.append(d)
    return json.dumps(specs, cls=OpEncoder)
```

**Pros:** 20 lines of code, handles everything.
**Cons:** `asdict()` is recursive and slow on large structures. No per-field customization.

### Option 6B: `to_spec()` method on each class

```python
@dataclass
class AddComponent:
    ...
    def to_spec(self) -> dict[str, Any]:
        d: dict[str, Any] = {"op": "add_component"}
        d["lib_reference"] = self.lib_reference
        if self.designator: d["designator"] = self.designator
        if self.pins: d["pins"] = [p.to_spec() for p in self.pins]
        ...
        return d
```

**Pros:** Full control, fast.
**Cons:** Boilerplate per class. Must update when fields change.

### Option 6C: Generic `to_spec()` via class metadata -- RECOMMENDED

```python
_OP_REGISTRY: dict[type, str] = {}

def op(tag: str):
    """Decorator that registers a dataclass as an op."""
    def decorator(cls):
        _OP_REGISTRY[cls] = tag
        return cls
    return decorator

@op("add_component")
@dataclass
class AddComponent:
    lib_reference: str
    ...

def to_spec(obj) -> dict[str, Any]:
    tag = _OP_REGISTRY.get(type(obj))
    d: dict[str, Any] = {"op": tag} if tag else {}
    for f in fields(obj):
        val = getattr(obj, f.name)
        if val is None:
            continue
        if isinstance(val, Enum):
            val = val.value
        elif isinstance(val, list):
            val = [
                to_spec(v) if hasattr(v, '__dataclass_fields__') else v
                for v in val
            ]
        elif hasattr(val, '__dataclass_fields__'):
            val = to_spec(val)
        d[f.name] = val
    return d
```

**Pros:** Generic, zero per-class boilerplate, easy to maintain.
**Cons:** Slight magic. Harder to customize edge cases.

### Verdict

**6C (generic with registry decorator)**. ~30 lines of code, handles everything,
adding a new op is just `@op("add_track")` on the class. Edge cases
(component_ref accepting Op objects) handled with a type check in the generic
function.

---

## Complete Op Inventory

All 29 `HighOp` variants and their Python dataclass equivalents.

### Component Management (SchDoc/SchLib)

```python
@op("add_component")
@dataclass
class AddComponent:
    lib_reference: str                              # required
    designator: str = ""
    value: str = ""
    description: str = ""
    pins: list[Pin] = field(default_factory=list)
    footprint: Footprint | None = None
    opid: str | None = None
    id: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_pin")
@dataclass
class AddPin:
    designator: str                                 # required
    name: str = ""
    electrical: PinElectrical | str = PinElectrical.PASSIVE
    at: tuple[int, int] = (0, 0)
    length_mils: int = 200
    rotation: int = 0                               # 0/90/180/270
    opid: str | None = None
    id: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_parameter")
@dataclass
class AddParameter:
    name: str                                       # required
    text: str                                       # required
    is_hidden: bool | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_alias")
@dataclass
class AddAlias:
    component_ref: str | Op | RefExpr               # required
    alias_name: str                                 # required
    opid: str | None = None

@op("remove_alias")
@dataclass
class RemoveAlias:
    component_ref: str | Op | RefExpr               # required
    alias_name: str                                 # required
    opid: str | None = None

@op("remove_component")
@dataclass
class RemoveComponent:
    component_ref: str | Op | RefExpr               # required
    opid: str | None = None

@op("edit_component")
@dataclass
class EditComponent:
    component_ref: str | Op | RefExpr               # required
    description: str | None = None
    part_count: int | None = None
    display_mode_count: int | None = None
    component_kind: int | None = None
    show_hidden_pins: bool | None = None
    opid: str | None = None

@op("edit_record")
@dataclass
class EditRecord:
    selector: RecordSelector                        # required
    patch: RecordPatch | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("remove_records")
@dataclass
class RemoveRecords:
    selector: RecordSelector                        # required
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None
```

### Queries (SchDoc/SchLib)

```python
@op("query")
@dataclass
class Query:
    selector: str                                   # required
    opid: str | None = None

@op("query_components")
@dataclass
class QueryComponents:
    pattern: str | None = None
    opid: str | None = None

@op("query_pins")
@dataclass
class QueryPins:
    component_ref: str | Op | RefExpr               # required
    opid: str | None = None

@op("query_records")
@dataclass
class QueryRecords:
    component_ref: str | Op | RefExpr               # required
    record_type: int | None = None
    opid: str | None = None
```

### Schematic Graphics (SchDoc/SchLib)

```python
@op("add_line")
@dataclass
class AddLine:
    from_: tuple[int, int]                          # required (serde: "from")
    to: tuple[int, int]                             # required
    color: int | None = None
    line_width: int | None = None
    line_style: LineStyle | int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_rectangle")
@dataclass
class AddRectangle:
    from_: tuple[int, int]                          # required
    to: tuple[int, int]                             # required
    color: int | None = None
    area_color: int | None = None
    is_solid: bool | None = None
    transparent: bool | None = None
    line_width: int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_arc")
@dataclass
class AddArc:
    cx_mils: int                                    # required
    cy_mils: int                                    # required
    radius_mils: int                                # required
    start_angle: float | None = None
    end_angle: float | None = None
    color: int | None = None
    line_width: int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_elliptical_arc")
@dataclass
class AddEllipticalArc:
    cx_mils: int                                    # required
    cy_mils: int                                    # required
    radius_mils: int                                # required
    secondary_radius_mils: int                      # required
    start_angle: float | None = None
    end_angle: float | None = None
    color: int | None = None
    line_width: int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_ellipse")
@dataclass
class AddEllipse:
    cx_mils: int                                    # required
    cy_mils: int                                    # required
    radius_mils: int                                # required
    secondary_radius_mils: int                      # required
    color: int | None = None
    area_color: int | None = None
    is_solid: bool | None = None
    line_width: int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_polyline")
@dataclass
class AddPolyline:
    points_mils: list[tuple[int, int]]              # required
    color: int | None = None
    line_width: int | None = None
    line_style: LineStyle | int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_polygon")
@dataclass
class AddPolygon:
    points_mils: list[tuple[int, int]]              # required
    color: int | None = None
    area_color: int | None = None
    is_solid: bool | None = None
    line_width: int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_bezier")
@dataclass
class AddBezier:
    points_mils: list[tuple[int, int]]              # required
    color: int | None = None
    line_width: int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_pie")
@dataclass
class AddPie:
    cx_mils: int                                    # required
    cy_mils: int                                    # required
    radius_mils: int                                # required
    start_angle: float | None = None
    end_angle: float | None = None
    color: int | None = None
    area_color: int | None = None
    is_solid: bool | None = None
    line_width: int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_round_rectangle")
@dataclass
class AddRoundRectangle:
    from_: tuple[int, int]                          # required
    to: tuple[int, int]                             # required
    corner_x_radius_mils: int                       # required
    corner_y_radius_mils: int                       # required
    color: int | None = None
    area_color: int | None = None
    is_solid: bool | None = None
    line_width: int | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_label")
@dataclass
class AddLabel:
    x_mils: int                                     # required
    y_mils: int                                     # required
    text: str                                       # required
    color: int | None = None
    font_id: int | None = None
    orientation: int | None = None
    justification: TextJustification | int | None = None
    is_mirrored: bool | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_text_frame")
@dataclass
class AddTextFrame:
    from_: tuple[int, int]                          # required
    to: tuple[int, int]                             # required
    text: str                                       # required
    color: int | None = None
    area_color: int | None = None
    font_id: int | None = None
    alignment: HorizontalAlign | int | None = None
    word_wrap: bool | None = None
    show_border: bool | None = None
    is_solid: bool | None = None
    clip_to_rect: bool | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None

@op("add_image")
@dataclass
class AddImage:
    from_: tuple[int, int]                          # required
    to: tuple[int, int]                             # required
    file_name: str                                  # required
    image_data: bytes | None = None
    keep_aspect: bool | None = None
    owner_part_id: int | None = None
    owner_part_display_mode: int | None = None
    opid: str | None = None
    component_ref: str | Op | RefExpr | None = None
```

### PCB Primitives (PcbDoc/PcbLib)

```python
@op("add_track")
@dataclass
class AddTrack:
    start: tuple[int, int]                          # required (mils)
    end: tuple[int, int]                            # required (mils)
    width_mils: int | None = None
    layer: str | None = None
    opid: str | None = None
    footprint_ref: str | Op | RefExpr | None = None

@op("add_via")
@dataclass
class AddVia:
    at: tuple[int, int]                             # required (mils)
    diameter_mils: int | None = None
    hole_size_mils: int | None = None
    from_layer: str | None = None
    to_layer: str | None = None
    opid: str | None = None
    footprint_ref: str | Op | RefExpr | None = None

@op("add_footprint")
@dataclass
class AddFootprint:
    name: str                                       # required
    pattern: str | None = None
    description: str | None = None
    opid: str | None = None
    id: str | None = None
```

### Supporting Types

```python
@dataclass
class Pin:
    designator: str                                 # required
    name: str = ""
    electrical: PinElectrical | str = PinElectrical.PASSIVE
    at: tuple[int, int] = (0, 0)
    length_mils: int = 200
    rotation: int = 0

@dataclass
class Footprint:
    model_name: str                                 # required
    map: list[FootprintMapEntry] = field(default_factory=list)

@dataclass
class FootprintMapEntry:
    pin: str                                        # required
    pad: str                                        # required

@dataclass
class RecordSelector:
    """One of: by_designator, by_record_type, by_index, by_name."""
    by_designator: str | None = None
    by_record_type: int | None = None
    by_index: int | None = None
    by_name: str | None = None

@dataclass
class RecordPatch:
    text: str | None = None
    name: str | None = None
    designator: str | None = None
    is_hidden: bool | None = None
    color: int | None = None
    line_width: int | None = None

@dataclass
class RefExpr:
    """Explicit reference expression for advanced cross-references."""
    op_id: str | None = None
    last: bool = False
    self_: bool = False
    sheet: bool = False
    steps: list[str | int] = field(default_factory=list)
```

### Op Union Type

```python
Op = (
    AddComponent | AddPin | AddParameter | AddAlias | RemoveAlias |
    RemoveComponent | EditComponent | EditRecord | RemoveRecords |
    Query | QueryComponents | QueryPins | QueryRecords |
    AddLine | AddRectangle | AddArc | AddEllipticalArc | AddEllipse |
    AddPolyline | AddPolygon | AddBezier | AddPie | AddRoundRectangle |
    AddLabel | AddTextFrame | AddImage |
    AddTrack | AddVia | AddFootprint
)
```

---

## End-to-End Usage Example

```python
import altium
from altium.ops import *

# Open an existing library
doc = altium.Document.open("MyLib.SchLib")

# Build a resistor component with symbol
comp = AddComponent(
    opid="r1",
    lib_reference="Resistor",
    designator="R?",
    value="10k",
    pins=[
        Pin("1", "A", PinElectrical.PASSIVE, at=(0, 100), length_mils=100),
        Pin("2", "B", PinElectrical.PASSIVE, at=(0, -100), length_mils=100,
            rotation=180),
    ],
    footprint=Footprint(
        model_name="R0805",
        map=[FootprintMapEntry("1", "1"), FootprintMapEntry("2", "2")],
    ),
)

# Draw the resistor body (rectangle)
body = AddRectangle(
    component_ref=comp,  # resolves to opid "r1"
    from_=(-50, 80),
    to=(50, -80),
    line_width=1,
    is_solid=False,
)

# Apply all ops
report = doc.apply([comp, body])

# Inspect results
for opid, result in report.results.items():
    print(f"  {opid}: {result.kind}")
    if result.warnings:
        for w in result.warnings:
            print(f"    WARNING: {w}")

# Save
doc.save("MyLib_modified.SchLib")
```

### Programmatic batch operations (the LLM agent use case)

```python
import altium
from altium.ops import *

doc = altium.Document.new_blank("SchLib")

# Generate a family of resistors from a BOM
for i, value in enumerate(["100", "1k", "10k", "100k"]):
    comp = AddComponent(
        opid=f"r{i}",
        lib_reference=f"R_{value}",
        designator="R?",
        value=value,
        pins=[
            Pin("1", "A", PinElectrical.PASSIVE, at=(0, 100), length_mils=100),
            Pin("2", "B", PinElectrical.PASSIVE, at=(0, -100), length_mils=100,
                rotation=180),
        ],
    )
    body = AddRectangle(
        component_ref=comp,
        from_=(-50, 80), to=(50, -80),
        line_width=1,
    )
    doc.apply([comp, body])

doc.save("ResistorFamily.SchLib")
```

### Query example

```python
doc = altium.Document.open("MyLib.SchLib")

# List all components
report = doc.apply([QueryComponents(opid="all")])
components = report.results["all"]
print(f"Found {len(components.refs)} components")
for ref in components.refs:
    print(f"  {ref.display_path}")

# Query pins of a specific component
report = doc.apply([
    QueryPins(opid="pins", component_ref="Resistor"),
])
```

### Raw JSON escape hatch

```python
import json

# For one-off ops or dynamically generated specs
doc.apply_json(json.dumps([
    {"op": "add_component", "lib_reference": "Cap", "designator": "C?"},
]))

# Or using the ops DSL directly
doc.apply_ops("""
    c1 = add_component(lib_reference: "Cap", designator: "C?")
""")
```

---

## Implementation Effort Estimate

| Component                      | Lines of code | Dependencies |
| ------------------------------ | ------------- | ------------ |
| Rust PyO3 crate                | ~200          | pyo3         |
| Python ops.py (dataclasses)    | ~300          | none         |
| Python enums.py                | ~80           | none         |
| Python serialization (to_spec) | ~50           | none         |
| Python __init__.py / re-exports| ~20           | none         |
| .pyi stubs for Rust types      | ~60           | none         |
| **Total**                      | **~710**      |              |

---

## Open Questions

1. **Single `Document` class vs separate `SchDoc`/`SchLib`/`PcbLib`/`PcbDoc`?**
   Single class is simpler but loses domain-specific methods. Separate classes
   give better type stubs but more Rust glue. Leaning toward single `Document`
   since domain validation already happens in the ops layer.

2. **Query result typing**: `OpResult.fields` is `dict[str, Any]`. Should we
   type-narrow query results (e.g. `QueryResult` with known fields), or keep
   as untyped dict?

3. **Package name**: `altium` (simple, might conflict) vs `altium-py` vs
   `pyaltium`?

4. **`from_` field naming**: Python keyword conflict. Use `from_` and remap
   to `"from"` during serialization? Or use `start`/`end` instead?

5. **Color representation**: Currently raw i32 (Win32 COLORREF `0x00BBGGRR`).
   Should Python expose a `Color` helper or accept hex strings like `"#FF0000"`?
