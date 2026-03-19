#!/bin/bash
# Roundtrip test all PcbDoc fixture files (save-as + semantic diff).
# Run with --help for usage.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FILE_TYPE="PcbDoc"
FILE_TYPE_LOWER="pcbdoc"
FILE_EXT="PcbDoc"
FIXTURE_DIR="data/pcbdoc"
source "$SCRIPT_DIR/_common.sh"
run_roundtrip "$@"
