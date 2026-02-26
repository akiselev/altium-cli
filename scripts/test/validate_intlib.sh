#!/bin/bash
# Validate all IntLib fixture files.
# Run with --help for usage.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FILE_TYPE="IntLib"
FILE_TYPE_LOWER="intlib"
FILE_EXT="IntLib"
FIXTURE_DIR="data/intlib"
source "$SCRIPT_DIR/_common.sh"
run_validate "$@"
