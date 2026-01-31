#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "olefile>=0.47",
#     "click>=8.0",
# ]
# ///
"""
Diff two Altium OLE/CFB files (SchLib, PcbLib, SchDoc, PcbDoc).

Compares the OLE stream tree structure and content of each stream,
with awareness of text vs binary streams. Provides subcommands for
summary diffs, text stream diffs, and hex diffs of binary streams.

Usage:
    ./scripts/diff-ole.py summary  file_a.SchLib file_b.SchLib
    ./scripts/diff-ole.py text     file_a.SchLib file_b.SchLib [--stream PATH]
    ./scripts/diff-ole.py hex      file_a.SchLib file_b.SchLib [--stream PATH]
    ./scripts/diff-ole.py full     file_a.SchLib file_b.SchLib
"""

from __future__ import annotations

import difflib
import sys
from typing import Optional

import click
import olefile


# ---------------------------------------------------------------------------
# Heuristics for classifying streams
# ---------------------------------------------------------------------------

# Streams known to contain pipe-delimited text parameters
TEXT_STREAM_SUFFIXES = (
    "data",
    "parameters",
    "widestrings",
)

# Streams that are almost always pure binary
BINARY_STREAM_NAMES = {
    "header",
    "primitiveguids",
    "embeddedfont",
    "embeddedfontdata",
}


def _stream_path(entry: list[str]) -> str:
    """Join OLE entry path components with /."""
    return "/".join(entry)


def _is_text_stream(path: str, data: bytes) -> bool:
    """Heuristic: decide whether a stream holds text parameters.

    Altium text streams are Windows-1252 pipe-delimited parameter blocks.
    They usually start with |HEADER= or |RECORD= or similar, or consist
    of mostly printable bytes with | separators.
    """
    lower = path.rsplit("/", 1)[-1].lower()

    if lower in BINARY_STREAM_NAMES:
        return False

    # Explicit suffix match
    if lower in TEXT_STREAM_SUFFIXES:
        # Even "data" streams can be binary (e.g. Tracks6/Data).
        # Check content: if >80% of bytes are printable ASCII/latin1, treat as text.
        pass

    if len(data) == 0:
        return True  # empty is "text" for display purposes

    # Content heuristic: count printable + whitespace bytes
    printable = sum(
        1 for b in data if (0x20 <= b < 0x7F) or b in (0x09, 0x0A, 0x0D, 0x00)
    )
    ratio = printable / len(data)
    return ratio > 0.80


# ---------------------------------------------------------------------------
# OLE reading helpers
# ---------------------------------------------------------------------------


def read_ole_streams(path: str) -> dict[str, bytes]:
    """Read all streams from an OLE file into {path: bytes}."""
    ole = olefile.OleFileIO(path)
    streams: dict[str, bytes] = {}
    for entry in ole.listdir(streams=True, storages=False):
        key = _stream_path(entry)
        try:
            streams[key] = ole.openstream(entry).read()
        except Exception:
            streams[key] = b""
    ole.close()
    return streams


# ---------------------------------------------------------------------------
# Formatting helpers
# ---------------------------------------------------------------------------

RESET = "\033[0m"
RED = "\033[31m"
GREEN = "\033[32m"
CYAN = "\033[36m"
YELLOW = "\033[33m"
DIM = "\033[2m"
BOLD = "\033[1m"


def _color(text: str, code: str) -> str:
    if not sys.stdout.isatty():
        return text
    return f"{code}{text}{RESET}"


def _decode_text(data: bytes) -> str:
    """Decode an Altium text stream, handling null bytes and Windows-1252."""
    # Strip trailing nulls, replace interior nulls with newlines (record separators)
    text = data.replace(b"\x00", b"\n")
    try:
        return text.decode("windows-1252")
    except Exception:
        return text.decode("latin-1", errors="replace")


def _split_params(text: str) -> list[str]:
    """Split decoded text into individual |KEY=VALUE lines for readable diffs."""
    lines: list[str] = []
    for raw_line in text.splitlines():
        # Split on | but keep it as a prefix for readability
        parts = raw_line.split("|")
        for part in parts:
            stripped = part.strip()
            if stripped:
                lines.append(f"|{stripped}")
    return lines


def _format_text_diff(path: str, data_a: bytes, data_b: bytes) -> list[str]:
    """Return unified diff lines for a text stream."""
    lines_a = _split_params(_decode_text(data_a))
    lines_b = _split_params(_decode_text(data_b))
    diff = list(
        difflib.unified_diff(
            lines_a,
            lines_b,
            fromfile=f"a/{path}",
            tofile=f"b/{path}",
            lineterm="",
        )
    )
    colored: list[str] = []
    for line in diff:
        if line.startswith("---") or line.startswith("+++"):
            colored.append(_color(line, BOLD))
        elif line.startswith("@@"):
            colored.append(_color(line, CYAN))
        elif line.startswith("-"):
            colored.append(_color(line, RED))
        elif line.startswith("+"):
            colored.append(_color(line, GREEN))
        else:
            colored.append(line)
    return colored


def _hex_line(offset: int, data: bytes) -> str:
    """Format 16 bytes as a hex dump line."""
    hex_parts = []
    ascii_parts = []
    for i in range(16):
        if i < len(data):
            hex_parts.append(f"{data[i]:02x}")
            ch = chr(data[i]) if 0x20 <= data[i] < 0x7F else "."
            ascii_parts.append(ch)
        else:
            hex_parts.append("  ")
            ascii_parts.append(" ")
        if i == 7:
            hex_parts.append("")  # extra space in middle
    return f"{offset:08x}  {' '.join(hex_parts)}  |{''.join(ascii_parts)}|"


def _format_hex_diff(
    path: str, data_a: bytes, data_b: bytes, context: int = 3
) -> list[str]:
    """Return a hex diff showing changed regions with context."""
    max_len = max(len(data_a), len(data_b))
    if max_len == 0:
        return []

    # Build hex dump lines for both
    lines_a: list[str] = []
    lines_b: list[str] = []
    for off in range(0, max_len, 16):
        chunk_a = data_a[off : off + 16] if off < len(data_a) else b""
        chunk_b = data_b[off : off + 16] if off < len(data_b) else b""
        lines_a.append(_hex_line(off, chunk_a))
        lines_b.append(_hex_line(off, chunk_b))

    diff = list(
        difflib.unified_diff(
            lines_a,
            lines_b,
            fromfile=f"a/{path}",
            tofile=f"b/{path}",
            lineterm="",
            n=context,
        )
    )
    colored: list[str] = []
    for line in diff:
        if line.startswith("---") or line.startswith("+++"):
            colored.append(_color(line, BOLD))
        elif line.startswith("@@"):
            colored.append(_color(line, CYAN))
        elif line.startswith("-"):
            colored.append(_color(line, RED))
        elif line.startswith("+"):
            colored.append(_color(line, GREEN))
        else:
            colored.append(line)
    return colored


def _size_str(n: int) -> str:
    if n < 1024:
        return f"{n} B"
    elif n < 1024 * 1024:
        return f"{n / 1024:.1f} KiB"
    else:
        return f"{n / (1024 * 1024):.1f} MiB"


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


@click.group()
def cli():
    """Diff two Altium OLE compound document files.

    Compares OLE stream trees and content of SchLib, PcbLib, SchDoc,
    and PcbDoc files.
    """
    pass


@cli.command()
@click.argument("file_a")
@click.argument("file_b")
def summary(file_a: str, file_b: str):
    """Show a structural summary of differences between two files.

    Lists streams that are added, removed, changed (with sizes), or identical.
    """
    streams_a = read_ole_streams(file_a)
    streams_b = read_ole_streams(file_b)

    all_paths = sorted(set(streams_a.keys()) | set(streams_b.keys()))

    added = []
    removed = []
    changed = []
    identical = []

    for path in all_paths:
        in_a = path in streams_a
        in_b = path in streams_b

        if in_a and not in_b:
            removed.append(path)
        elif in_b and not in_a:
            added.append(path)
        elif streams_a[path] == streams_b[path]:
            identical.append(path)
        else:
            changed.append(path)

    # Print report
    click.echo(_color(f"--- {file_a}", BOLD))
    click.echo(_color(f"+++ {file_b}", BOLD))
    click.echo()

    if removed:
        click.echo(_color("Removed streams:", RED))
        for p in removed:
            click.echo(_color(f"  - {p}  ({_size_str(len(streams_a[p]))})", RED))
        click.echo()

    if added:
        click.echo(_color("Added streams:", GREEN))
        for p in added:
            click.echo(_color(f"  + {p}  ({_size_str(len(streams_b[p]))})", GREEN))
        click.echo()

    if changed:
        click.echo(_color("Changed streams:", YELLOW))
        for p in changed:
            da, db = streams_a[p], streams_b[p]
            kind = (
                "text" if _is_text_stream(p, da) or _is_text_stream(p, db) else "binary"
            )
            click.echo(
                _color(
                    f"  ~ {p}  ({_size_str(len(da))} -> {_size_str(len(db))})  [{kind}]",
                    YELLOW,
                )
            )
        click.echo()

    if identical:
        click.echo(_color(f"Identical streams: {len(identical)}", DIM))
        for p in identical:
            click.echo(_color(f"    {p}  ({_size_str(len(streams_a[p]))})", DIM))
        click.echo()

    # Summary line
    total = len(all_paths)
    click.echo(
        f"{total} streams total: "
        f"{_color(f'{len(added)} added', GREEN)}, "
        f"{_color(f'{len(removed)} removed', RED)}, "
        f"{_color(f'{len(changed)} changed', YELLOW)}, "
        f"{len(identical)} identical"
    )


@cli.command()
@click.argument("file_a")
@click.argument("file_b")
@click.option(
    "--stream",
    "-s",
    "stream_filter",
    default=None,
    help="Only show diff for this stream path (substring match).",
)
def text(file_a: str, file_b: str, stream_filter: Optional[str]):
    """Show text diffs of parameter/text streams.

    Altium text streams contain pipe-delimited |KEY=VALUE parameters.
    This command decodes them and shows a unified diff with each parameter
    on its own line for readability.
    """
    streams_a = read_ole_streams(file_a)
    streams_b = read_ole_streams(file_b)
    all_paths = sorted(set(streams_a.keys()) | set(streams_b.keys()))

    found_any = False
    for path in all_paths:
        if stream_filter and stream_filter.lower() not in path.lower():
            continue

        da = streams_a.get(path, b"")
        db = streams_b.get(path, b"")

        if da == db:
            continue

        # Only show text streams in this subcommand
        if not (_is_text_stream(path, da) or _is_text_stream(path, db)):
            continue

        diff_lines = _format_text_diff(path, da, db)
        if diff_lines:
            found_any = True
            click.echo()
            for line in diff_lines:
                click.echo(line)

    if not found_any:
        click.echo("No text stream differences found.")


@cli.command()
@click.argument("file_a")
@click.argument("file_b")
@click.option(
    "--stream",
    "-s",
    "stream_filter",
    default=None,
    help="Only show diff for this stream path (substring match).",
)
@click.option("--context", "-C", default=3, help="Number of context lines in hex diff.")
@click.option(
    "--all",
    "show_all",
    is_flag=True,
    help="Show hex diff for ALL streams, not just binary ones.",
)
def hex(
    file_a: str, file_b: str, stream_filter: Optional[str], context: int, show_all: bool
):
    """Show hex diffs of binary streams.

    Displays a unified diff of hex dumps for binary streams (tracks, pads,
    primitives, headers, etc.). Use --all to include text streams too.
    """
    streams_a = read_ole_streams(file_a)
    streams_b = read_ole_streams(file_b)
    all_paths = sorted(set(streams_a.keys()) | set(streams_b.keys()))

    found_any = False
    for path in all_paths:
        if stream_filter and stream_filter.lower() not in path.lower():
            continue

        da = streams_a.get(path, b"")
        db = streams_b.get(path, b"")

        if da == db:
            continue

        # In default mode, skip text streams
        if not show_all and (_is_text_stream(path, da) and _is_text_stream(path, db)):
            continue

        diff_lines = _format_hex_diff(path, da, db, context=context)
        if diff_lines:
            found_any = True
            click.echo()
            kind = "text" if _is_text_stream(path, da) else "binary"
            click.echo(
                _color(
                    f"[{kind}] {path}  ({_size_str(len(da))} vs {_size_str(len(db))})",
                    BOLD,
                )
            )
            for line in diff_lines:
                click.echo(line)

    if not found_any:
        click.echo("No binary stream differences found.")


@cli.command()
@click.argument("file_a")
@click.argument("file_b")
@click.option(
    "--stream",
    "-s",
    "stream_filter",
    default=None,
    help="Only show diff for this stream path (substring match).",
)
@click.option("--context", "-C", default=3, help="Number of context lines in hex diff.")
def full(file_a: str, file_b: str, stream_filter: Optional[str], context: int):
    """Show full diff: text diffs for text streams, hex diffs for binary streams.

    Combines the 'text' and 'hex' subcommands into a single output,
    automatically choosing the right format for each stream.
    """
    streams_a = read_ole_streams(file_a)
    streams_b = read_ole_streams(file_b)
    all_paths = sorted(set(streams_a.keys()) | set(streams_b.keys()))

    # Summary header
    added = [p for p in all_paths if p not in streams_a]
    removed = [p for p in all_paths if p not in streams_b]

    if removed:
        for p in removed:
            click.echo(
                _color(f"Stream removed: {p}  ({_size_str(len(streams_a[p]))})", RED)
            )
    if added:
        for p in added:
            click.echo(
                _color(f"Stream added:   {p}  ({_size_str(len(streams_b[p]))})", GREEN)
            )

    found_any = False
    for path in all_paths:
        if stream_filter and stream_filter.lower() not in path.lower():
            continue

        da = streams_a.get(path, b"")
        db = streams_b.get(path, b"")

        if da == db:
            continue

        is_text = _is_text_stream(path, da) or _is_text_stream(path, db)

        if is_text:
            diff_lines = _format_text_diff(path, da, db)
        else:
            diff_lines = _format_hex_diff(path, da, db, context=context)

        if diff_lines:
            found_any = True
            kind = "text" if is_text else "binary"
            click.echo()
            click.echo(
                _color(
                    f"[{kind}] {path}  ({_size_str(len(da))} -> {_size_str(len(db))})",
                    BOLD,
                )
            )
            for line in diff_lines:
                click.echo(line)

    if not found_any:
        click.echo("Files are identical.")


@cli.command()
@click.argument("file_path")
def tree(file_path: str):
    """List all OLE streams in a file (not a diff — just inspection)."""
    streams = read_ole_streams(file_path)
    for path in sorted(streams.keys()):
        data = streams[path]
        kind = "text" if _is_text_stream(path, data) else "bin "
        click.echo(f"  [{kind}]  {_size_str(len(data)):>10}  {path}")


if __name__ == "__main__":
    cli()
