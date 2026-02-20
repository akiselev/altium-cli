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
    32, 33, 34, 37, 39, 41, 43, 44, 45, 46, 47, 48, 209, 225,
}
IMPLEMENTED_PCB_OBJECT_IDS = {1, 2, 3, 4, 5, 6, 7, 11, 12}


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

PCB_LIB_SYSTEM_TOP_LEVEL = {
    "SectionKeys",
    "FileHeader",
    "Library",
    "FileVersionInfo",
}

PCBDOC_PRIMITIVE_SECTION_OBJECT_IDS = {
    "arcs6": 1,
    "pads6": 2,
    "vias6": 3,
    "tracks6": 4,
    "texts6": 5,
    "fills6": 6,
    "connections6": 7,
    "regions6": 11,
    "shapebasedregions6": 11,
    "splitplaneregions6": 11,
    "componentbodies6": 12,
    "shapebasedcomponentbodies6": 12,
    # Legacy primitive storages still seen in AD26 exports.
    "boardregions": 11,
    "texts": 5,
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
    # Altium uses pipes and equals for parameters.
    # Null bytes are often used for padding but we allow them in printable sum.
    printable = sum(
        1 for b in data if (32 <= b <= 126) or b in (0, 9, 10, 13) or (160 <= b <= 255)
    )
    ratio = printable / len(data)
    # Heuristic: Altium parameter blocks are very high in printable chars
    if ratio < 0.60:
        return False
    sample = _decode_text(data[: min(8192, len(data))])
    # Must have pipes to be considered Altium param text
    return "|" in sample


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


def parse_param_records(data: bytes) -> list[list[tuple[str, str]]]:
    # Rust mirrors: don't replace \x00 with \n, and don't splitlines.
    # Preserve raw segment whitespace and duplicate keys (ordered pairs).
    text = _decode_text(data)
    records: list[list[tuple[str, str]]] = []

    # Handle %UTF8% prefix decoding similarly to Rust's decode_utf8_from_win1252
    def decode_value(k: str, v: str) -> tuple[str, str]:
        if k.startswith("%UTF8%"):
            real_key = k[6:]
            try:
                # Re-encode to win1252 then decode as utf8
                raw = v.encode("windows-1252", errors="replace")
                return real_key, raw.decode("utf-8", errors="replace")
            except Exception:
                return real_key, v
        return k, v

    current_rec: list[tuple[str, str]] = []
    has_record_key = False
    for seg in text.split("|"):
        if seg == "":
            continue
        if "=" in seg:
            k, v = seg.split("=", 1)
            k, v = decode_value(k, v)
        else:
            # Solo value (key is empty string, like Rust)
            k, v = "", seg

        if k.upper() == "RECORD" and has_record_key:
            records.append(current_rec)
            current_rec = []
            has_record_key = False

        current_rec.append((k, v))
        if k.upper() == "RECORD":
            has_record_key = True

    if current_rec:
        records.append(current_rec)
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
        # Increased range to be more future-proof, Altium uses up to ~220 for some sch records
        # but PCB IDs are usually smaller. Let's allow up to 255.
        ids.append(type_id)
        # Pad=2 (6 subrecords), Text=5 (2 subrecords)
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


def parse_pcbdoc_data_object_ids(path: str, data: bytes) -> tuple[list[int], str | None]:
    # Primitive sections are in <Section>/Data streams.
    parts = path.split("/")
    if len(parts) != 2 or parts[1].lower() != "data":
        return [], None

    section = parts[0].lower()
    expected_id = PCBDOC_PRIMITIVE_SECTION_OBJECT_IDS.get(section)
    if expected_id is None:
        return [], None

    pos = 0
    ids: list[int] = []
    while pos < len(data):
        type_id = data[pos]
        pos += 1
        ids.append(type_id)

        if type_id != expected_id:
            return ids, f"unexpected-object-id:{type_id}:expected:{expected_id}"

        # Pad=2 (6 subrecords), Text=5 (2 subrecords)
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


def _read_ole(path: Path) -> tuple[bool, list[list[str]], dict[str, bytes], dict[str, str], str | None]:
    try:
        ole = olefile.OleFileIO(str(path))
    except NotOleFileError:
        # Some Altium formats (notably some .PrjPcb files) are plain text.
        raw = path.read_bytes()
        return False, [], {"(raw)": raw}, {}, None
    except Exception as exc:
        return False, [], {}, {}, f"{type(exc).__name__}: {exc}"

    storages = ole.listdir(streams=False, storages=True)
    streams: dict[str, bytes] = {}
    stream_read_errors: dict[str, str] = {}
    for entry in ole.listdir(streams=True, storages=False):
        key = _stream_path(entry)
        try:
            streams[key] = ole.openstream(entry).read()
        except Exception as exc:
            stream_read_errors[key] = f"{type(exc).__name__}: {exc}"
    ole.close()
    return True, storages, streams, stream_read_errors, None


def _match_stream(path: str, stream_filter: str | None) -> bool:
    if not stream_filter:
        return True
    return stream_filter.lower() in path.lower()


def cmd_container(args: argparse.Namespace) -> int:
    path = Path(args.path)
    is_ole, storages, streams, stream_read_errors, read_error = _read_ole(path)
    all_stream_names = sorted(set(streams.keys()) | set(stream_read_errors.keys()))
    out: dict[str, Any] = {
        "file": str(path),
        "is_ole": is_ole,
        "read_error": read_error,
        "storage_count": len(storages),
        "stream_count": len(all_stream_names),
        "stream_read_error_count": len(stream_read_errors),
        "stream_read_errors": dict(sorted(stream_read_errors.items())),
        "storages": sorted(_stream_path(s) for s in storages),
        "streams": [],
    }
    for sp in all_stream_names:
        if not _match_stream(sp, args.stream):
            continue
        if sp in stream_read_errors:
            out["streams"].append(
                {
                    "path": sp,
                    "size": None,
                    "kind": "unreadable",
                    "block_count": None,
                    "block_parse_error": None,
                    "preview_hex": "",
                    "preview_text": "",
                    "read_error": stream_read_errors[sp],
                }
            )
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
                "read_error": None,
            }
        )

    if args.json:
        print(json.dumps(out, indent=2 if args.pretty else None))
        return 1 if read_error or stream_read_errors else 0

    print(f"FILE: {path}")
    print(
        f"OLE: {is_ole}  Storages: {len(storages)}  Streams: {len(all_stream_names)}"
        f"  ReadErrors: {len(stream_read_errors)}"
    )
    if read_error:
        print(f"  read_error={read_error}")
    for st in out["streams"]:
        if st["read_error"]:
            print(f"  [error ] {'-':>8}  read_error={st['read_error']}  {st['path']}")
            continue
        block_part = f"blocks={st['block_count']}" + (f" ({st['block_parse_error']})" if st["block_parse_error"] else "")
        print(
            f"  [{st['kind']:<6}] {st['size']:>8}  {block_part:<28}  {st['path']}"
        )
    return 1 if read_error or stream_read_errors else 0


def cmd_blocks(args: argparse.Namespace) -> int:
    path = Path(args.path)
    _is_ole, _storages, streams, stream_read_errors, read_error = _read_ole(path)
    result: dict[str, Any] = {
        "file": str(path),
        "read_error": read_error,
        "stream_read_error_count": len(stream_read_errors),
        "stream_read_errors": dict(sorted(stream_read_errors.items())),
        "streams": [],
    }

    for sp in sorted(streams):
        if not _match_stream(sp, args.stream):
            continue
        data = streams[sp]
        blocks, err = parse_size_prefixed_blocks(data)
        if args.only_valid and err is not None:
            continue
        if args.max_blocks is None:
            selected_blocks = [asdict(b) for b in blocks]
        else:
            selected_blocks = [asdict(b) for b in blocks[: args.max_blocks]]

        stream_entry: dict[str, Any] = {
            "path": sp,
            "size": len(data),
            "kind": _classify_stream(sp, data),
            "block_parse_error": err,
            "blocks": selected_blocks,
        }
        result["streams"].append(stream_entry)
    for sp, err in sorted(stream_read_errors.items()):
        if not _match_stream(sp, args.stream):
            continue
        result["streams"].append(
            {
                "path": sp,
                "size": None,
                "kind": "unreadable",
                "block_parse_error": None,
                "read_error": err,
                "blocks": [],
            }
        )

    if args.json:
        print(json.dumps(result, indent=2 if args.pretty else None))
        return 1 if read_error or stream_read_errors else 0

    print(f"FILE: {path}")
    if read_error:
        print(f"read_error={read_error}")
    for st in result["streams"]:
        if st.get("read_error"):
            print(f"\n{st['path']}  unreadable error={st['read_error']}")
            continue
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
    return 1 if read_error or stream_read_errors else 0


def _pairs_to_multimap(pairs: list[tuple[str, str]]) -> dict[str, list[str]]:
    out: dict[str, list[str]] = defaultdict(list)
    for k, v in pairs:
        out[k].append(v)
    return dict(out)


def _record_id_from_pairs(pairs: list[tuple[str, str]]) -> int | None:
    for k, v in pairs:
        if k.upper() == "RECORD":
            try:
                return int(v)
            except Exception:
                return None
    return None


def _collect_text_records_for_stream(path: str, data: bytes) -> tuple[list[dict[str, Any]], list[str]]:
    records: list[dict[str, Any]] = []
    warnings: list[str] = []
    blocks, err = parse_size_prefixed_blocks(data)

    # Force text parsing if it's a known text stream
    is_known_text = _classify_stream(path, data) == "text"

    if err is None and blocks:
        skipped_non_text_blocks = 0
        for b in blocks:
            block_data_start = b.offset + 4
            payload = data[block_data_start : block_data_start + b.size]
            if not is_known_text and not _is_probably_text(payload):
                skipped_non_text_blocks += 1
                continue
            for pairs in parse_param_records(payload):
                record_id = _record_id_from_pairs(pairs)
                records.append(
                    {
                        "stream": path,
                        "block_index": b.index,
                        "record_id": record_id,
                        "params_pairs": [{"key": k, "value": v} for k, v in pairs],
                        "params_multi": _pairs_to_multimap(pairs),
                    }
                )
        if skipped_non_text_blocks:
            warnings.append(f"skipped_non_text_blocks={skipped_non_text_blocks}")
        return records, warnings

    # Fallback: whole stream as text
    if err is not None:
        warnings.append(f"block_parse_error={err};fallback=whole-stream")
    if is_known_text or _is_probably_text(data):
        for pairs in parse_param_records(data):
            record_id = _record_id_from_pairs(pairs)
            records.append(
                {
                    "stream": path,
                    "block_index": None,
                    "record_id": record_id,
                    "params_pairs": [{"key": k, "value": v} for k, v in pairs],
                    "params_multi": _pairs_to_multimap(pairs),
                }
            )
    else:
        warnings.append("stream_not_text_by_classifier")
    return records, warnings


def cmd_text(args: argparse.Namespace) -> int:
    path = Path(args.path)
    _is_ole, _storages, streams, stream_read_errors, read_error = _read_ole(path)
    all_records: list[dict[str, Any]] = []
    key_counts: Counter[str] = Counter()
    record_counts: Counter[int] = Counter()
    text_parse_warnings: dict[str, list[str]] = {}

    for sp in sorted(streams):
        if not _match_stream(sp, args.stream):
            continue
        stream_records, warnings = _collect_text_records_for_stream(sp, streams[sp])
        if warnings:
            text_parse_warnings[sp] = warnings
        for rec in stream_records:
            all_records.append(rec)
            if rec["record_id"] is not None:
                record_counts[rec["record_id"]] += 1
            for param in rec["params_pairs"]:
                k = param["key"]
                key_counts[k] += 1

    if args.records:
        if args.max_records is None:
            selected_records = all_records
        else:
            selected_records = all_records[: args.max_records]
    else:
        selected_records = []

    output: dict[str, Any] = {
        "file": str(path),
        "read_error": read_error,
        "stream_read_error_count": len(stream_read_errors),
        "stream_read_errors": dict(sorted(stream_read_errors.items())),
        "text_parse_warning_count": sum(len(v) for v in text_parse_warnings.values()),
        "text_parse_warnings": dict(sorted(text_parse_warnings.items())),
        "total_text_records": len(all_records),
        "record_id_counts": dict(sorted(record_counts.items())),
        "top_keys": key_counts.most_common(args.top_keys),
        "records": selected_records,
    }

    if args.json:
        print(json.dumps(output, indent=2 if args.pretty else None))
        return 1 if read_error or stream_read_errors else 0

    print(f"FILE: {path}")
    if read_error:
        print(f"read_error={read_error}")
    if stream_read_errors:
        print("Stream read errors:")
        for sp, err in sorted(stream_read_errors.items()):
            if _match_stream(sp, args.stream):
                print(f"  {sp}: {err}")
    if text_parse_warnings:
        print("Text parse warnings:")
        for sp, warnings in sorted(text_parse_warnings.items()):
            print(f"  {sp}: {', '.join(warnings)}")
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
                for param in rec["params_pairs"]:
                    print(f"    {param['key']}={param['value']}")
    return 1 if read_error or stream_read_errors else 0


def _collect_pcb_object_ids(
    path: str,
    data: bytes,
    ext: str,
    all_stream_names: set[str] | None = None,
) -> tuple[list[int], list[str]]:
    ids: list[int] = []
    warnings: list[str] = []
    lower = path.lower()
    path_parts = path.split("/")

    if ext == ".pcblib" and not (len(path_parts) == 2 and path_parts[1].lower() == "data"):
        return ids, warnings

    if ext == ".pcblib":
        parent = path.rsplit("/", 1)[0] if "/" in path else ""
        top_level = parent.split("/", 1)[0] if parent else ""
        if top_level in PCB_LIB_SYSTEM_TOP_LEVEL:
            return ids, warnings
        if all_stream_names is not None:
            # Typical PcbLib footprint storage must have Parameters or Header
            if f"{parent}/Parameters" not in all_stream_names and f"{parent}/Header" not in all_stream_names:
                warnings.append("missing-footprint-siblings:Parameters/Header")
        parsed, err = parse_pcblib_data_object_ids(data)
        if err is not None:
            warnings.append(f"pcblib-data-parse-error:{err}")
        return parsed, warnings

    if ext == ".pcbdoc":
        parsed, err = parse_pcbdoc_data_object_ids(path, data)
        if err is not None:
            warnings.append(f"pcbdoc-data-parse-error:{err}")
        return parsed, warnings

    return ids, warnings


def cmd_pcb(args: argparse.Namespace) -> int:
    path = Path(args.path)
    ext = path.suffix.lower()
    _is_ole, _storages, streams, stream_read_errors, read_error = _read_ole(path)

    by_stream: dict[str, list[int]] = {}
    parse_issues: dict[str, list[str]] = {}
    counts: Counter[int] = Counter()
    stream_names = set(streams.keys())
    for sp in sorted(streams):
        if not _match_stream(sp, args.stream):
            continue
        ids, warnings = _collect_pcb_object_ids(sp, streams[sp], ext, stream_names)
        if warnings:
            parse_issues[sp] = warnings
        if not ids:
            continue
        by_stream[sp] = ids
        counts.update(ids)

    if args.max_ids_per_stream is None:
        stream_ids = {k: v for k, v in by_stream.items()}
    else:
        stream_ids = {k: v[: args.max_ids_per_stream] for k, v in by_stream.items()}

    output = {
        "file": str(path),
        "read_error": read_error,
        "stream_read_error_count": len(stream_read_errors),
        "stream_read_errors": dict(sorted(stream_read_errors.items())),
        "parse_issue_count": sum(len(v) for v in parse_issues.values()),
        "parse_issues": dict(sorted(parse_issues.items())),
        "object_id_counts": dict(sorted(counts.items())),
        "streams": stream_ids,
    }
    if args.json:
        print(json.dumps(output, indent=2 if args.pretty else None))
        return 1 if read_error or stream_read_errors else 0

    print(f"FILE: {path}")
    if read_error:
        print(f"read_error={read_error}")
    if stream_read_errors:
        print("Stream read errors:")
        for sp, err in sorted(stream_read_errors.items()):
            if _match_stream(sp, args.stream):
                print(f"  {sp}: {err}")
    if parse_issues:
        print("PCB parse issues:")
        for sp, warnings in sorted(parse_issues.items()):
            print(f"  {sp}: {', '.join(warnings)}")
    if not counts:
        print("No PCB object IDs found.")
        return 1 if read_error or stream_read_errors else 0
    print("Object IDs:")
    for oid, c in sorted(counts.items()):
        print(f"  ID={oid:<3} count={c}")
    if args.stream_ids:
        print("\nPer-stream IDs:")
        for sp, ids in output["streams"].items():
            print(f"  {sp}: {ids}")
    return 1 if read_error or stream_read_errors else 0


def _scan_one_file(path: Path) -> dict[str, Any]:
    ext = path.suffix.lower()
    is_ole, storages, streams, stream_read_errors, read_error = _read_ole(path)

    stream_entries = []
    sch_record_counts: Counter[int] = Counter()
    pcb_object_counts: Counter[int] = Counter()
    all_record_keys: Counter[str] = Counter()
    text_parse_warnings: dict[str, list[str]] = {}
    pcb_parse_issues: dict[str, list[str]] = {}

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

        text_records, warnings = _collect_text_records_for_stream(sp, data)
        if warnings:
            text_parse_warnings[sp] = warnings
        for rec in text_records:
            rid = rec["record_id"]
            if rid is not None:
                sch_record_counts[rid] += 1
            for param in rec["params_pairs"]:
                all_record_keys[param["key"]] += 1

        pcb_ids, pcb_warnings = _collect_pcb_object_ids(sp, data, ext, stream_names)
        if pcb_warnings:
            pcb_parse_issues[sp] = pcb_warnings
        pcb_object_counts.update(pcb_ids)

    return {
        "file": str(path),
        "extension": ext,
        "is_ole": is_ole,
        "read_error": read_error,
        "stream_read_errors": dict(sorted(stream_read_errors.items())),
        "storage_count": len(storages),
        "stream_count": len(streams) + len(stream_read_errors),
        "streams": stream_entries,
        "schematic_record_counts": dict(sorted(sch_record_counts.items())),
        "pcb_object_counts": dict(sorted(pcb_object_counts.items())),
        "top_param_keys": all_record_keys.most_common(30),
        "text_parse_warnings": dict(sorted(text_parse_warnings.items())),
        "pcb_parse_issues": dict(sorted(pcb_parse_issues.items())),
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
    read_error_files = [
        s["file"] for s in scans if s.get("read_error") or s.get("stream_read_errors")
    ]

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
            "read_error_files": read_error_files,
        },
    }

    if args.json:
        print(json.dumps(summary, indent=2 if args.pretty else None))
        return 1 if read_error_files else 0

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
    if read_error_files:
        print("\nFiles with read errors:")
        for f in read_error_files:
            print(f"  {f}")
        return 1
    return 0


def _first_diff_offset(a: bytes, b: bytes) -> int | None:
    n = min(len(a), len(b))
    for i in range(n):
        if a[i] != b[i]:
            return i
    if len(a) != len(b):
        return n
    return None


def _canonicalize_param_record_order_agnostic(
    pairs: list[tuple[str, str]],
) -> tuple[tuple[str, str], ...]:
    # Case-insensitive keys, preserve duplicate key/value entries.
    return tuple(sorted((k.upper(), v) for k, v in pairs))


def _canonicalize_param_payload_order_agnostic(
    data: bytes,
) -> list[tuple[tuple[str, str], ...]]:
    return [
        _canonicalize_param_record_order_agnostic(pairs)
        for pairs in parse_param_records(data)
    ]


def _stream_equal_param_order_agnostic(path: str, a: bytes, b: bytes) -> bool:
    if a == b:
        return True

    a_blocks, a_err = parse_size_prefixed_blocks(a)
    b_blocks, b_err = parse_size_prefixed_blocks(b)

    # Block-framed payloads (e.g. SchLib Data): compare block-by-block while
    # allowing key-order-insensitive param comparison inside text payloads.
    if a_err is None and b_err is None and (a_blocks or b_blocks):
        if len(a_blocks) != len(b_blocks):
            return False
        for ba, bb in zip(a_blocks, b_blocks):
            pa = a[ba.offset + 4 : ba.offset + 4 + ba.size]
            pb = b[bb.offset + 4 : bb.offset + 4 + bb.size]
            if ba.flags != bb.flags:
                return False

            # SchLib Data streams use non-zero block flags for binary records
            # (notably legacy pin blocks). Keep those as strict byte compares.
            if ba.flags != 0:
                if pa != pb:
                    return False
                continue

            a_text = _is_probably_text(pa)
            b_text = _is_probably_text(pb)
            if a_text and b_text:
                if (
                    _canonicalize_param_payload_order_agnostic(pa)
                    != _canonicalize_param_payload_order_agnostic(pb)
                ):
                    return False
            elif a_text != b_text or pa != pb:
                return False
        return True

    # Whole-stream text payloads.
    if _classify_stream(path, a) == "text" and _classify_stream(path, b) == "text":
        return _canonicalize_param_payload_order_agnostic(a) == _canonicalize_param_payload_order_agnostic(b)

    return False


def cmd_compare(args: argparse.Namespace) -> int:
    original_path = Path(args.original)
    rebuilt_path = Path(args.rebuilt)

    (
        orig_is_ole,
        orig_storages,
        orig_streams,
        orig_stream_read_errors,
        orig_read_error,
    ) = _read_ole(original_path)
    (
        rebuilt_is_ole,
        rebuilt_storages,
        rebuilt_streams,
        rebuilt_stream_read_errors,
        rebuilt_read_error,
    ) = _read_ole(rebuilt_path)

    orig_storage_names = {_stream_path(s) for s in orig_storages}
    rebuilt_storage_names = {_stream_path(s) for s in rebuilt_storages}
    storage_names = sorted(orig_storage_names | rebuilt_storage_names)
    if args.stream:
        storage_names = [n for n in storage_names if _match_stream(n, args.stream)]
    only_in_original_storages = sorted(
        n for n in storage_names if n in orig_storage_names and n not in rebuilt_storage_names
    )
    only_in_rebuilt_storages = sorted(
        n for n in storage_names if n in rebuilt_storage_names and n not in orig_storage_names
    )

    orig_names = set(orig_streams.keys()) | set(orig_stream_read_errors.keys())
    rebuilt_names = set(rebuilt_streams.keys()) | set(rebuilt_stream_read_errors.keys())
    all_names = sorted(orig_names | rebuilt_names)
    if args.stream:
        all_names = [n for n in all_names if _match_stream(n, args.stream)]

    only_in_original = sorted(n for n in all_names if n in orig_names and n not in rebuilt_names)
    only_in_rebuilt = sorted(n for n in all_names if n in rebuilt_names and n not in orig_names)
    different_streams: list[dict[str, Any]] = []
    unreadable_streams: list[dict[str, str | None]] = []
    matched_streams: list[str] = []

    for name in all_names:
        if name in only_in_original or name in only_in_rebuilt:
            continue

        orig_err = orig_stream_read_errors.get(name)
        rebuilt_err = rebuilt_stream_read_errors.get(name)
        if orig_err or rebuilt_err:
            unreadable_streams.append(
                {
                    "path": name,
                    "original_error": orig_err,
                    "rebuilt_error": rebuilt_err,
                }
            )
            if orig_err != rebuilt_err:
                different_streams.append(
                    {
                        "path": name,
                        "kind": "unreadable",
                        "original_error": orig_err,
                        "rebuilt_error": rebuilt_err,
                    }
                )
            continue

        orig_data = orig_streams[name]
        rebuilt_data = rebuilt_streams[name]
        if orig_data == rebuilt_data:
            matched_streams.append(name)
            continue
        if args.param_order_agnostic and _stream_equal_param_order_agnostic(
            name, orig_data, rebuilt_data
        ):
            matched_streams.append(name)
            continue

        different_streams.append(
            {
                "path": name,
                "kind": _classify_stream(name, orig_data),
                "original_size": len(orig_data),
                "rebuilt_size": len(rebuilt_data),
                "first_diff_offset": _first_diff_offset(orig_data, rebuilt_data),
            }
        )

    output = {
        "original": str(original_path),
        "rebuilt": str(rebuilt_path),
        "original_is_ole": orig_is_ole,
        "rebuilt_is_ole": rebuilt_is_ole,
        "original_read_error": orig_read_error,
        "rebuilt_read_error": rebuilt_read_error,
        "original_stream_read_errors": dict(sorted(orig_stream_read_errors.items())),
        "rebuilt_stream_read_errors": dict(sorted(rebuilt_stream_read_errors.items())),
        "only_in_original_storages": only_in_original_storages,
        "only_in_rebuilt_storages": only_in_rebuilt_storages,
        "only_in_original": only_in_original,
        "only_in_rebuilt": only_in_rebuilt,
        "different_streams": different_streams,
        "unreadable_stream_count": len(unreadable_streams),
        "unreadable_streams": unreadable_streams,
        "matched_count": len(matched_streams),
        "param_order_agnostic": bool(args.param_order_agnostic),
        "different_count": (
            len(only_in_original_storages)
            + len(only_in_rebuilt_storages)
            + len(only_in_original)
            + len(only_in_rebuilt)
            + len(different_streams)
        ),
    }

    if args.show_matched:
        output["matched_streams"] = matched_streams

    if args.json:
        print(json.dumps(output, indent=2 if args.pretty else None))
    else:
        print(f"ORIGINAL: {original_path}")
        print(f"REBUILT:  {rebuilt_path}")
        if orig_read_error:
            print(f"original_read_error={orig_read_error}")
        if rebuilt_read_error:
            print(f"rebuilt_read_error={rebuilt_read_error}")
        if orig_stream_read_errors:
            print("Original stream read errors:")
            for sp, err in sorted(orig_stream_read_errors.items()):
                print(f"  {sp}: {err}")
        if rebuilt_stream_read_errors:
            print("Rebuilt stream read errors:")
            for sp, err in sorted(rebuilt_stream_read_errors.items()):
                print(f"  {sp}: {err}")
        print(f"Only in original storages: {len(only_in_original_storages)}")
        for sp in only_in_original_storages:
            print(f"  - {sp}")
        print(f"Only in rebuilt storages:  {len(only_in_rebuilt_storages)}")
        for sp in only_in_rebuilt_storages:
            print(f"  + {sp}")
        print(f"Unreadable streams: {len(unreadable_streams)}")
        for d in unreadable_streams:
            print(
                f"  ! {d['path']} orig={d['original_error']} rebuilt={d['rebuilt_error']}"
            )
        print(f"Matched streams: {len(matched_streams)}")
        print(f"Only in original: {len(only_in_original)}")
        for sp in only_in_original:
            print(f"  - {sp}")
        print(f"Only in rebuilt:  {len(only_in_rebuilt)}")
        for sp in only_in_rebuilt:
            print(f"  + {sp}")
        print(f"Different streams: {len(different_streams)}")
        for d in different_streams:
            if d["kind"] == "unreadable":
                print(
                    f"  * {d['path']} unreadable "
                    f"orig={d['original_error']} rebuilt={d['rebuilt_error']}"
                )
            else:
                print(
                    f"  * {d['path']} kind={d['kind']} "
                    f"orig={d['original_size']} rebuilt={d['rebuilt_size']} "
                    f"first_diff={d['first_diff_offset']}"
                )

    return (
        1
        if (
            output["different_count"]
            or output["unreadable_stream_count"]
            or orig_read_error
            or rebuilt_read_error
        )
        else 0
    )


def cmd_everything(args: argparse.Namespace) -> int:
    # Compose outputs for one file without forcing JSON-only usage.
    path = args.path
    base = argparse.Namespace(path=path, stream=args.stream, json=False, pretty=False)

    status = 0
    print("=== CONTAINER ===")
    status |= cmd_container(base)

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
    status |= cmd_blocks(blocks_args)

    print("\n=== TEXT ===")
    text_args = argparse.Namespace(
        path=path,
        stream=args.stream,
        json=False,
        pretty=False,
        top_keys=30,
        records=False,
        show_params=False,
        max_records=None,
    )
    status |= cmd_text(text_args)

    print("\n=== PCB ===")
    pcb_args = argparse.Namespace(
        path=path,
        stream=args.stream,
        json=False,
        pretty=False,
        max_ids_per_stream=None,
        stream_ids=True,
    )
    status |= cmd_pcb(pcb_args)
    return 1 if status else 0


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
    sp_blocks.add_argument(
        "--max-blocks",
        type=int,
        default=None,
        help="Max blocks to show per stream (default: all).",
    )
    sp_blocks.add_argument("--preview", action="store_true", help="Show payload text/hex previews.")
    add_json_flags(sp_blocks)
    sp_blocks.set_defaults(func=cmd_blocks)

    sp_text = sub.add_parser("text", help="Parse pipe-delimited key=value records from text streams/blocks.")
    sp_text.add_argument("path", help="Path to an Altium OLE file.")
    sp_text.add_argument("--stream", help="Substring filter for stream path.")
    sp_text.add_argument("--top-keys", type=int, default=40, help="Number of most common keys to show.")
    sp_text.add_argument("--records", action="store_true", help="Include parsed records.")
    sp_text.add_argument("--show-params", action="store_true", help="When used with --records, print all key/value pairs.")
    sp_text.add_argument(
        "--max-records",
        type=int,
        default=None,
        help="Max records to include when --records is set (default: all).",
    )
    add_json_flags(sp_text)
    sp_text.set_defaults(func=cmd_text)

    sp_pcb = sub.add_parser("pcb", help="Extract PCB object IDs from primitive streams.")
    sp_pcb.add_argument("path", help="Path to a .PcbDoc or .PcbLib file.")
    sp_pcb.add_argument("--stream", help="Substring filter for stream path.")
    sp_pcb.add_argument("--stream-ids", action="store_true", help="Show per-stream object ID lists.")
    sp_pcb.add_argument(
        "--max-ids-per-stream",
        type=int,
        default=None,
        help="Max IDs to include per stream when --stream-ids is set (default: all).",
    )
    add_json_flags(sp_pcb)
    sp_pcb.set_defaults(func=cmd_pcb)

    sp_scan = sub.add_parser("scan", help="Scan a file or directory and report coverage/unimplemented IDs.")
    sp_scan.add_argument("path", help="File or directory (e.g. data/).")
    add_json_flags(sp_scan)
    sp_scan.set_defaults(func=cmd_scan)

    sp_compare = sub.add_parser("compare", help="Compare two Altium files stream-by-stream without heuristics.")
    sp_compare.add_argument("original", help="Original/reference file.")
    sp_compare.add_argument("rebuilt", help="Rebuilt/candidate file.")
    sp_compare.add_argument("--stream", help="Substring filter for stream path.")
    sp_compare.add_argument(
        "--param-order-agnostic",
        action="store_true",
        help="Compare text/param payloads ignoring per-record key order.",
    )
    sp_compare.add_argument("--show-matched", action="store_true", help="Include matched stream names in output.")
    add_json_flags(sp_compare)
    sp_compare.set_defaults(func=cmd_compare)

    sp_everything = sub.add_parser("everything", help="Run container+blocks+text+pcb for one file.")
    sp_everything.add_argument("path", help="Path to an Altium OLE file.")
    sp_everything.add_argument("--stream", help="Substring filter for stream path.")
    sp_everything.add_argument(
        "--max-blocks",
        type=int,
        default=None,
        help="Max blocks printed per stream (default: all).",
    )
    sp_everything.set_defaults(func=cmd_everything)

    return p


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
