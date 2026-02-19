#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = [
#   "olefile>=0.47",
# ]
# ///
"""
Inspect Altium OLE/CFB files at container, block, text-record, and PCB-object levels.

This tool is model-oriented and mirrors docs/model:
- container-format.md  -> `container`, `blocks`
- schematic-records.md -> `text`
- pcb-records.md       -> `pcb`

`scan` runs everything across a file or directory and reports coverage vs known
implemented IDs in the current v2 Rust model.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import zlib
from collections import Counter, defaultdict
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Any

import olefile
from olefile.olefile import NotOleFileError


# docs/model/schematic-records.md
MODEL_SCHEMATIC_RECORD_IDS = {
    1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 14, 17, 18, 22, 25, 26, 27, 28, 29, 30, 31,
    34, 37, 41, 43, 44, 45, 46, 47, 48, 209,
}

# docs/model/pcb-records.md
MODEL_PCB_OBJECT_IDS = {1, 2, 3, 4, 5, 6, 10, 11, 12, 13, 14}

# Current v2 record coverage (crates/altium-format/src/v2/records)
IMPLEMENTED_SCH_RECORD_IDS = {
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 17, 18, 22, 25, 26, 27, 28, 29, 30, 31,
    32, 33, 34, 37, 39, 40, 41, 44, 45, 209, 215,
}
IMPLEMENTED_PCB_OBJECT_IDS = {1, 2, 3, 4, 5, 6, 9, 11, 12}


TEXT_STREAM_SUFFIXES = {
    "fileheader",
    "additional",
    "parameters",
    "widestrings",
    "board6/data",
    "components6/data",
    "nets6/data",
    "rules6/data",
    "classes6/data",
}


@dataclass
class BlockInfo:
    index: int
    offset: int
    raw_header: int
    flags: int
    size: int
    payload_preview_hex: str
    payload_preview_text: str
    compressed: bool
    compressed_id: str | None = None
    compressed_size: int | None = None


def _stream_path(parts: list[str]) -> str:
    return "/".join(parts)


def _decode_text(data: bytes) -> str:
    return data.decode("windows-1252", errors="replace")


def _text_preview(data: bytes, max_len: int = 80) -> str:
    s = _decode_text(data[: max_len * 2]).replace("\x00", "\\0").replace("\n", "\\n")
    return s[:max_len]


def _hex_preview(data: bytes, max_len: int = 24) -> str:
    return data[:max_len].hex(" ")


def _is_probably_text(data: bytes) -> bool:
    if not data:
        return True
    printable = sum(
        1 for b in data if (32 <= b <= 126) or b in (9, 10, 13, 0) or (160 <= b <= 255)
    )
    ratio = printable / len(data)
    if ratio < 0.80:
        return False
    sample = _decode_text(data[: min(4096, len(data))])
    return ("|" in sample and "=" in sample) or ratio > 0.92


def _classify_stream(path: str, data: bytes) -> str:
    lower = path.lower()
    if lower in TEXT_STREAM_SUFFIXES or any(lower.endswith("/" + s) for s in TEXT_STREAM_SUFFIXES):
        return "text"
    if _is_probably_text(data):
        return "text"
    return "binary"


def parse_size_prefixed_blocks(data: bytes) -> tuple[list[BlockInfo], str | None]:
    blocks: list[BlockInfo] = []
    off = 0
    index = 0
    if not data:
        return blocks, None

    while off < len(data):
        if off + 4 > len(data):
            return blocks, f"trailing-bytes:{len(data) - off}"
        raw_header = int.from_bytes(data[off : off + 4], "little")
        flags = (raw_header >> 24) & 0xFF
        size = raw_header & 0x00FFFFFF
        off += 4
        if off + size > len(data):
            return blocks, f"block-overflow:index={index}:size={size}:remaining={len(data)-off}"
        payload = data[off : off + size]
        compressed, comp_id, comp_size = _inspect_compressed_payload(payload)
        blocks.append(
            BlockInfo(
                index=index,
                offset=off - 4,
                raw_header=raw_header,
                flags=flags,
                size=size,
                payload_preview_hex=_hex_preview(payload),
                payload_preview_text=_text_preview(payload),
                compressed=compressed,
                compressed_id=comp_id,
                compressed_size=comp_size,
            )
        )
        off += size
        index += 1

    return blocks, None


def _inspect_compressed_payload(payload: bytes) -> tuple[bool, str | None, int | None]:
    # Altium compressed storage payload:
    # [0xD0][u8 id_len][id bytes][u32 block_header][compressed bytes...]
    if len(payload) < 7 or payload[0] != 0xD0:
        return False, None, None

    id_len = payload[1]
    id_start = 2
    id_end = id_start + id_len
    if id_end + 4 > len(payload):
        return False, None, None
    ident = _decode_text(payload[id_start:id_end])
    hdr = int.from_bytes(payload[id_end : id_end + 4], "little")
    comp_len = hdr & 0x00FFFFFF
    comp_start = id_end + 4
    comp_end = comp_start + comp_len
    if comp_end > len(payload):
        return True, ident, None

    comp = payload[comp_start:comp_end]
    # Rust reader skips first 2 bytes before inflate; try both.
    out_len = None
    try:
        out_len = len(zlib.decompress(comp))
    except Exception:
        if len(comp) > 2:
            try:
                out_len = len(zlib.decompress(comp[2:], -zlib.MAX_WBITS))
            except Exception:
                out_len = None
    return True, ident, out_len


def parse_param_records(data: bytes) -> list[dict[str, str]]:
    text = _decode_text(data).replace("\x00", "\n")
    records: list[dict[str, str]] = []
    for line in text.splitlines():
        line = line.strip()
        if not line or "|" not in line or "=" not in line:
            continue
        rec: dict[str, str] = {}
        for seg in line.split("|"):
            seg = seg.strip()
            if not seg or "=" not in seg:
                continue
            k, v = seg.split("=", 1)
            rec[k] = v
        if rec:
            records.append(rec)
    return records


def parse_pcblib_data_object_ids(data: bytes) -> tuple[list[int], str | None]:
    # Format from v2/documents/pcblib.rs:
    # u32 pattern_len, pattern_bytes, then [u8 type][subrecords...]
    # subrecord framing: u32 len + bytes
    pos = 0
    if len(data) < 4:
        return [], "too-short"
    pat_len = int.from_bytes(data[pos : pos + 4], "little")
    pos += 4
    if pos + pat_len > len(data):
        return [], "bad-pattern-len"
    pos += pat_len
    ids: list[int] = []
    while pos < len(data):
        type_id = data[pos]
        pos += 1
        if type_id > 25:
            return ids, f"type-out-of-range:{type_id}"
        ids.append(type_id)
        n = 6 if type_id == 2 else 2 if type_id == 5 else 1
        for _ in range(n):
            if pos + 4 > len(data):
                return ids, "truncated-subrecord-len"
            sub_len = int.from_bytes(data[pos : pos + 4], "little")
            pos += 4
            if pos + sub_len > len(data):
                return ids, "truncated-subrecord-data"
            pos += sub_len
    return ids, None


def _is_altium_path(path: Path) -> bool:
    return path.suffix.lower() in {".schlib", ".schdoc", ".pcblib", ".pcbdoc", ".prjpcb", ".intlib"}


def _iter_input_files(target: Path) -> list[Path]:
    if target.is_file():
        return [target]
    files = sorted(p for p in target.rglob("*") if p.is_file() and _is_altium_path(p))
    return files


def _read_ole(path: Path) -> tuple[bool, list[list[str]], dict[str, bytes]]:
    try:
        ole = olefile.OleFileIO(str(path))
    except NotOleFileError:
        # Some Altium formats (notably some .PrjPcb files) are plain text.
        raw = path.read_bytes()
        return False, [], {"(raw)": raw}

    storages = ole.listdir(streams=False, storages=True)
    streams: dict[str, bytes] = {}
    for entry in ole.listdir(streams=True, storages=False):
        key = _stream_path(entry)
        try:
            streams[key] = ole.openstream(entry).read()
        except Exception:
            streams[key] = b""
    ole.close()
    return True, storages, streams


def _match_stream(path: str, stream_filter: str | None) -> bool:
    if not stream_filter:
        return True
    return stream_filter.lower() in path.lower()


def cmd_container(args: argparse.Namespace) -> int:
    path = Path(args.path)
    is_ole, storages, streams = _read_ole(path)
    out: dict[str, Any] = {
        "file": str(path),
        "is_ole": is_ole,
        "storage_count": len(storages),
        "stream_count": len(streams),
        "storages": sorted(_stream_path(s) for s in storages),
        "streams": [],
    }
    for sp in sorted(streams):
        if not _match_stream(sp, args.stream):
            continue
        data = streams[sp]
        blocks, err = parse_size_prefixed_blocks(data)
        out["streams"].append(
            {
                "path": sp,
                "size": len(data),
                "kind": _classify_stream(sp, data),
                "block_count": len(blocks),
                "block_parse_error": err,
                "preview_hex": _hex_preview(data),
                "preview_text": _text_preview(data),
            }
        )

    if args.json:
        print(json.dumps(out, indent=2 if args.pretty else None))
        return 0

    print(f"FILE: {path}")
    print(f"OLE: {is_ole}  Storages: {len(storages)}  Streams: {len(streams)}")
    for st in out["streams"]:
        block_part = f"blocks={st['block_count']}" + (f" ({st['block_parse_error']})" if st["block_parse_error"] else "")
        print(
            f"  [{st['kind']:<6}] {st['size']:>8}  {block_part:<28}  {st['path']}"
        )
    return 0


def cmd_blocks(args: argparse.Namespace) -> int:
    path = Path(args.path)
    _is_ole, _storages, streams = _read_ole(path)
    result: dict[str, Any] = {"file": str(path), "streams": []}

    for sp in sorted(streams):
        if not _match_stream(sp, args.stream):
            continue
        data = streams[sp]
        blocks, err = parse_size_prefixed_blocks(data)
        if args.only_valid and err is not None:
            continue
        stream_entry: dict[str, Any] = {
            "path": sp,
            "size": len(data),
            "kind": _classify_stream(sp, data),
            "block_parse_error": err,
            "blocks": [asdict(b) for b in blocks[: args.max_blocks]],
        }
        result["streams"].append(stream_entry)

    if args.json:
        print(json.dumps(result, indent=2 if args.pretty else None))
        return 0

    print(f"FILE: {path}")
    for st in result["streams"]:
        print(
            f"\n{st['path']}  size={st['size']} kind={st['kind']} "
            f"block_error={st['block_parse_error']}"
        )
        for b in st["blocks"]:
            comp = ""
            if b["compressed"]:
                comp = f" compressed id={b['compressed_id']} uncompressed={b['compressed_size']}"
            print(
                f"  #{b['index']:04d} off=0x{b['offset']:x} hdr=0x{b['raw_header']:08x} "
                f"flags=0x{b['flags']:02x} size={b['size']}{comp}"
            )
            if args.preview:
                print(f"      hex:  {b['payload_preview_hex']}")
                print(f"      text: {b['payload_preview_text']}")
    return 0


def _collect_text_records_for_stream(path: str, data: bytes) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    blocks, err = parse_size_prefixed_blocks(data)
    if err is None and blocks:
        for b in blocks:
            block_data_start = b.offset + 4
            payload = data[block_data_start : block_data_start + b.size]
            if not _is_probably_text(payload):
                continue
            for rec in parse_param_records(payload):
                record_id = None
                if "RECORD" in rec:
                    try:
                        record_id = int(rec["RECORD"])
                    except Exception:
                        record_id = None
                records.append(
                    {
                        "stream": path,
                        "block_index": b.index,
                        "record_id": record_id,
                        "params": rec,
                    }
                )
        return records

    # Fallback: whole stream as text
    if _is_probably_text(data):
        for rec in parse_param_records(data):
            record_id = None
            if "RECORD" in rec:
                try:
                    record_id = int(rec["RECORD"])
                except Exception:
                    record_id = None
            records.append(
                {
                    "stream": path,
                    "block_index": None,
                    "record_id": record_id,
                    "params": rec,
                }
            )
    return records


def cmd_text(args: argparse.Namespace) -> int:
    path = Path(args.path)
    _is_ole, _storages, streams = _read_ole(path)
    all_records: list[dict[str, Any]] = []
    key_counts: Counter[str] = Counter()
    record_counts: Counter[int] = Counter()

    for sp in sorted(streams):
        if not _match_stream(sp, args.stream):
            continue
        for rec in _collect_text_records_for_stream(sp, streams[sp]):
            all_records.append(rec)
            if rec["record_id"] is not None:
                record_counts[rec["record_id"]] += 1
            for k in rec["params"]:
                key_counts[k] += 1

    output: dict[str, Any] = {
        "file": str(path),
        "total_text_records": len(all_records),
        "record_id_counts": dict(sorted(record_counts.items())),
        "top_keys": key_counts.most_common(args.top_keys),
        "records": all_records[: args.max_records] if args.records else [],
    }

    if args.json:
        print(json.dumps(output, indent=2 if args.pretty else None))
        return 0

    print(f"FILE: {path}")
    print(f"Total parsed text records: {len(all_records)}")
    if record_counts:
        print("Record IDs:")
        for rid, count in sorted(record_counts.items()):
            print(f"  RECORD={rid:<4} count={count}")
    else:
        print("Record IDs: none")
    if key_counts:
        print(f"Top {args.top_keys} keys:")
        for k, c in key_counts.most_common(args.top_keys):
            print(f"  {k:<32} {c}")

    if args.records:
        print("\nSample records:")
        for rec in output["records"]:
            rid = rec["record_id"]
            print(f"  stream={rec['stream']} block={rec['block_index']} RECORD={rid}")
            if args.show_params:
                for k, v in rec["params"].items():
                    print(f"    {k}={v}")
    return 0


def _collect_pcb_object_ids(
    path: str,
    data: bytes,
    ext: str,
    all_stream_names: set[str] | None = None,
) -> list[int]:
    ids: list[int] = []
    lower = path.lower()

    if ext == ".pcblib" and lower.endswith("/data"):
        parent = path.rsplit("/", 1)[0] if "/" in path else ""
        if all_stream_names is not None:
            if f"{parent}/Parameters" not in all_stream_names or f"{parent}/Header" not in all_stream_names:
                return ids
        parsed, err = parse_pcblib_data_object_ids(data)
        if err is not None:
            return ids
        return parsed

    blocks, err = parse_size_prefixed_blocks(data)
    if err is not None or not blocks:
        return ids
    if ext == ".pcbdoc" and _classify_stream(path, data) != "binary":
        return ids

    is_probably_primitive_stream = (
        lower.endswith("/data")
        and any(
            tok in lower
            for tok in (
                "tracks6", "arcs6", "fills6", "pads6", "vias6",
                "texts6", "regions6", "polygons6", "dimensions6", "coordinates6",
                "componentbodies6", "primitives6",
            )
        )
    )
    if not is_probably_primitive_stream and ext != ".pcbdoc":
        return ids

    for bi, b in enumerate(blocks):
        block_data_start = b.offset + 4
        payload = data[block_data_start : block_data_start + b.size]
        if not payload:
            continue
        # PcbDoc primitive streams often have block0 = u32 count.
        if bi == 0 and len(payload) == 4 and ext == ".pcbdoc":
            continue
        ids.append(payload[0])

    return ids


def cmd_pcb(args: argparse.Namespace) -> int:
    path = Path(args.path)
    ext = path.suffix.lower()
    _is_ole, _storages, streams = _read_ole(path)

    by_stream: dict[str, list[int]] = {}
    counts: Counter[int] = Counter()
    stream_names = set(streams.keys())
    for sp in sorted(streams):
        if not _match_stream(sp, args.stream):
            continue
        ids = _collect_pcb_object_ids(sp, streams[sp], ext, stream_names)
        if not ids:
            continue
        by_stream[sp] = ids
        counts.update(ids)

    output = {
        "file": str(path),
        "object_id_counts": dict(sorted(counts.items())),
        "streams": {k: v[: args.max_ids_per_stream] for k, v in by_stream.items()},
    }
    if args.json:
        print(json.dumps(output, indent=2 if args.pretty else None))
        return 0

    print(f"FILE: {path}")
    if not counts:
        print("No PCB object IDs found.")
        return 0
    print("Object IDs:")
    for oid, c in sorted(counts.items()):
        print(f"  ID={oid:<3} count={c}")
    if args.stream_ids:
        print("\nPer-stream IDs:")
        for sp, ids in output["streams"].items():
            print(f"  {sp}: {ids}")
    return 0


def _scan_one_file(path: Path) -> dict[str, Any]:
    ext = path.suffix.lower()
    is_ole, storages, streams = _read_ole(path)

    stream_entries = []
    sch_record_counts: Counter[int] = Counter()
    pcb_object_counts: Counter[int] = Counter()
    all_record_keys: Counter[str] = Counter()

    stream_names = set(streams.keys())
    for sp in sorted(streams):
        data = streams[sp]
        blocks, block_err = parse_size_prefixed_blocks(data)
        kind = _classify_stream(sp, data)
        stream_entries.append(
            {
                "path": sp,
                "size": len(data),
                "kind": kind,
                "block_count": len(blocks),
                "block_parse_error": block_err,
            }
        )

        for rec in _collect_text_records_for_stream(sp, data):
            rid = rec["record_id"]
            if rid is not None:
                sch_record_counts[rid] += 1
            for k in rec["params"]:
                all_record_keys[k] += 1

        pcb_ids = _collect_pcb_object_ids(sp, data, ext, stream_names)
        pcb_object_counts.update(pcb_ids)

    return {
        "file": str(path),
        "extension": ext,
        "is_ole": is_ole,
        "storage_count": len(storages),
        "stream_count": len(streams),
        "streams": stream_entries,
        "schematic_record_counts": dict(sorted(sch_record_counts.items())),
        "pcb_object_counts": dict(sorted(pcb_object_counts.items())),
        "top_param_keys": all_record_keys.most_common(30),
    }


def cmd_scan(args: argparse.Namespace) -> int:
    target = Path(args.path)
    files = _iter_input_files(target)
    if not files:
        print(f"No Altium files found under: {target}", file=sys.stderr)
        return 2

    scans = [_scan_one_file(p) for p in files]

    all_sch_counts: Counter[int] = Counter()
    all_pcb_counts: Counter[int] = Counter()
    stream_names: Counter[str] = Counter()
    for scan in scans:
        all_sch_counts.update(scan["schematic_record_counts"])
        all_pcb_counts.update(scan["pcb_object_counts"])
        for st in scan["streams"]:
            stream_names[st["path"]] += 1

    observed_sch = set(all_sch_counts.keys())
    observed_pcb = set(all_pcb_counts.keys())

    summary = {
        "input": str(target),
        "files_scanned": len(scans),
        "files": scans,
        "aggregate": {
            "schematic_record_counts": dict(sorted(all_sch_counts.items())),
            "pcb_object_counts": dict(sorted(all_pcb_counts.items())),
            "common_streams": stream_names.most_common(200),
            "coverage": {
                "docs_model_schematic_missing_in_code": sorted(
                    MODEL_SCHEMATIC_RECORD_IDS - IMPLEMENTED_SCH_RECORD_IDS
                ),
                "docs_model_pcb_missing_in_code": sorted(
                    MODEL_PCB_OBJECT_IDS - IMPLEMENTED_PCB_OBJECT_IDS
                ),
                "observed_schematic_missing_in_code": sorted(
                    observed_sch - IMPLEMENTED_SCH_RECORD_IDS
                ),
                "observed_pcb_missing_in_code": sorted(
                    observed_pcb - IMPLEMENTED_PCB_OBJECT_IDS
                ),
            },
        },
    }

    if args.json:
        print(json.dumps(summary, indent=2 if args.pretty else None))
        return 0

    print(f"Scanned {len(scans)} files under {target}")
    print("\nAggregate schematic RECORD IDs:")
    for rid, c in sorted(all_sch_counts.items()):
        print(f"  RECORD={rid:<4} count={c}")
    print("\nAggregate PCB object IDs:")
    for oid, c in sorted(all_pcb_counts.items()):
        print(f"  ID={oid:<3} count={c}")

    cov = summary["aggregate"]["coverage"]
    print("\nCoverage gaps vs docs/model:")
    print(f"  Schematic docs IDs missing in code: {cov['docs_model_schematic_missing_in_code']}")
    print(f"  PCB docs IDs missing in code:       {cov['docs_model_pcb_missing_in_code']}")
    print(f"  Observed schematic IDs missing:     {cov['observed_schematic_missing_in_code']}")
    print(f"  Observed PCB IDs missing:           {cov['observed_pcb_missing_in_code']}")
    return 0


def cmd_everything(args: argparse.Namespace) -> int:
    # Compose outputs for one file without forcing JSON-only usage.
    path = args.path
    base = argparse.Namespace(path=path, stream=args.stream, json=False, pretty=False)

    print("=== CONTAINER ===")
    cmd_container(base)

    print("\n=== BLOCKS ===")
    blocks_args = argparse.Namespace(
        path=path,
        stream=args.stream,
        json=False,
        pretty=False,
        only_valid=False,
        max_blocks=args.max_blocks,
        preview=True,
    )
    cmd_blocks(blocks_args)

    print("\n=== TEXT ===")
    text_args = argparse.Namespace(
        path=path,
        stream=args.stream,
        json=False,
        pretty=False,
        top_keys=30,
        records=False,
        show_params=False,
        max_records=20,
    )
    cmd_text(text_args)

    print("\n=== PCB ===")
    pcb_args = argparse.Namespace(
        path=path,
        stream=args.stream,
        json=False,
        pretty=False,
        max_ids_per_stream=100,
        stream_ids=True,
    )
    cmd_pcb(pcb_args)
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="Inspect Altium OLE/CFB internals (container, blocks, records, objects).")
    sub = p.add_subparsers(dest="cmd", required=True)

    def add_json_flags(sp: argparse.ArgumentParser) -> None:
        sp.add_argument("--json", action="store_true", help="Emit JSON output.")
        sp.add_argument("--pretty", action="store_true", help="Pretty-print JSON.")

    sp_container = sub.add_parser("container", help="List storages/streams and stream-level metadata.")
    sp_container.add_argument("path", help="Path to an Altium OLE file.")
    sp_container.add_argument("--stream", help="Substring filter for stream path.")
    add_json_flags(sp_container)
    sp_container.set_defaults(func=cmd_container)

    sp_blocks = sub.add_parser("blocks", help="Decode 24-bit-size/8-bit-flag block framing for each stream.")
    sp_blocks.add_argument("path", help="Path to an Altium OLE file.")
    sp_blocks.add_argument("--stream", help="Substring filter for stream path.")
    sp_blocks.add_argument("--only-valid", action="store_true", help="Show only streams that parse cleanly as block streams.")
    sp_blocks.add_argument("--max-blocks", type=int, default=2000, help="Max blocks to show per stream.")
    sp_blocks.add_argument("--preview", action="store_true", help="Show payload text/hex previews.")
    add_json_flags(sp_blocks)
    sp_blocks.set_defaults(func=cmd_blocks)

    sp_text = sub.add_parser("text", help="Parse pipe-delimited key=value records from text streams/blocks.")
    sp_text.add_argument("path", help="Path to an Altium OLE file.")
    sp_text.add_argument("--stream", help="Substring filter for stream path.")
    sp_text.add_argument("--top-keys", type=int, default=40, help="Number of most common keys to show.")
    sp_text.add_argument("--records", action="store_true", help="Include sample records.")
    sp_text.add_argument("--show-params", action="store_true", help="When used with --records, print all key/value pairs.")
    sp_text.add_argument("--max-records", type=int, default=30, help="Max sample records.")
    add_json_flags(sp_text)
    sp_text.set_defaults(func=cmd_text)

    sp_pcb = sub.add_parser("pcb", help="Extract PCB object IDs from primitive streams.")
    sp_pcb.add_argument("path", help="Path to a .PcbDoc or .PcbLib file.")
    sp_pcb.add_argument("--stream", help="Substring filter for stream path.")
    sp_pcb.add_argument("--stream-ids", action="store_true", help="Show per-stream object ID lists.")
    sp_pcb.add_argument("--max-ids-per-stream", type=int, default=200, help="Cap per-stream ID list output.")
    add_json_flags(sp_pcb)
    sp_pcb.set_defaults(func=cmd_pcb)

    sp_scan = sub.add_parser("scan", help="Scan a file or directory and report coverage/unimplemented IDs.")
    sp_scan.add_argument("path", help="File or directory (e.g. data/).")
    add_json_flags(sp_scan)
    sp_scan.set_defaults(func=cmd_scan)

    sp_everything = sub.add_parser("everything", help="Run container+blocks+text+pcb for one file.")
    sp_everything.add_argument("path", help="Path to an Altium OLE file.")
    sp_everything.add_argument("--stream", help="Substring filter for stream path.")
    sp_everything.add_argument("--max-blocks", type=int, default=120, help="Max blocks printed per stream.")
    sp_everything.set_defaults(func=cmd_everything)

    return p


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
