#!/bin/bash
# Roundtrip test all SchLib fixture files (save-as + semantic diff).
# Run with --help for usage.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FILE_TYPE="SchLib"
FILE_TYPE_LOWER="schlib"
FILE_EXT="SchLib"
FIXTURE_DIR="data/schlib"
source "$SCRIPT_DIR/_common.sh"
run_roundtrip "$@"
