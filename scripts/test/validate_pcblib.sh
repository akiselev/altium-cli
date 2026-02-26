#!/bin/bash
# Validate all PcbLib fixture files.
# Run with --help for usage.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FILE_TYPE="PcbLib"
FILE_TYPE_LOWER="pcblib"
FILE_EXT="PcbLib"
FIXTURE_DIR="data/pcblib"
source "$SCRIPT_DIR/_common.sh"
run_validate "$@"
