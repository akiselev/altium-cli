#!/bin/bash
# Validate all SchDoc fixture files.
# Run with --help for usage.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FILE_TYPE="SchDoc"
FILE_TYPE_LOWER="schdoc"
FILE_EXT="SchDoc"
FIXTURE_DIR="data/schdoc"
source "$SCRIPT_DIR/_common.sh"
run_validate "$@"
