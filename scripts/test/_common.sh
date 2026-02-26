#!/bin/bash
# Shared library for validate_*.sh and roundtrip_*.sh scripts.
# Source this file; do not execute directly.
#
# Protocol:
#   1. The calling script sets configuration variables (FILE_TYPE, FIXTURE_DIR, etc.)
#   2. The calling script sources this file
#   3. The calling script calls run_validate "$@" or run_roundtrip "$@"

set -euo pipefail

# ── Defaults ──────────────────────────────────────────────────────────────────
CLI="${ALTIUM_CLI:-altium-cli}"
MAX_FILES=0          # 0 = unlimited
VERBOSE=false
STOP_ON_ERROR=false
PATTERN=""
NO_COLOR=false
REVERSE=false
KEEP_DIR=""          # roundtrip: directory to keep outputs (empty = auto-clean)
EXTRA_ARGS=()        # leftover positional args (explicit file paths)

# ── Color helpers ─────────────────────────────────────────────────────────────
_setup_colors() {
  if $NO_COLOR || [ ! -t 1 ]; then
    RED=""; GREEN=""; YELLOW=""; BOLD=""; DIM=""; RESET=""
  else
    RED=$'\033[0;31m'
    GREEN=$'\033[0;32m'
    YELLOW=$'\033[0;33m'
    BOLD=$'\033[1m'
    DIM=$'\033[2m'
    RESET=$'\033[0m'
  fi
}

# ── Argument parsing ─────────────────────────────────────────────────────────
_parse_args() {
  # Pre-scan for --no-color so help output respects it
  for arg in "$@"; do
    [ "$arg" = "--no-color" ] && NO_COLOR=true
  done
  _setup_colors

  while [ $# -gt 0 ]; do
    case "$1" in
      -h|--help)
        _show_help
        exit 0
        ;;
      -n|--max)
        MAX_FILES="$2"; shift 2
        ;;
      -v|--verbose)
        VERBOSE=true; shift
        ;;
      -s|--stop-on-error)
        STOP_ON_ERROR=true; shift
        ;;
      -p|--pattern)
        PATTERN="$2"; shift 2
        ;;
      -r|--reverse)
        REVERSE=true; shift
        ;;
      --no-color)
        NO_COLOR=true; shift
        ;;
      --keep)
        if [ $# -ge 2 ] && [[ ! "$2" =~ ^- ]]; then
          KEEP_DIR="$2"; shift 2
        else
          KEEP_DIR="__auto__"; shift
        fi
        ;;
      --)
        shift; EXTRA_ARGS+=("$@"); break
        ;;
      -*)
        echo "Unknown option: $1" >&2
        echo "Run with --help for usage." >&2
        exit 2
        ;;
      *)
        EXTRA_ARGS+=("$1"); shift
        ;;
    esac
  done
  _setup_colors
}

# ── Help (overridden per-mode) ───────────────────────────────────────────────
_show_help_validate() {
  cat <<EOF
${BOLD}validate_${FILE_TYPE_LOWER}.sh${RESET} - Validate all ${FILE_TYPE} fixture files

${BOLD}USAGE${RESET}
    ./scripts/test/validate_${FILE_TYPE_LOWER}.sh [OPTIONS] [FILE...]

${BOLD}DESCRIPTION${RESET}
    Runs \`altium validate\` on fixture files from ${FIXTURE_DIR}/.
    Files are sorted by size (smallest first) for fast failure on simple
    files. Pass explicit FILE paths to override fixture discovery.

${BOLD}OPTIONS${RESET}
    -h, --help            Show this help and exit
    -n, --max N           Process only the first N files (after sorting)
    -v, --verbose         Show output for passing files too
    -s, --stop-on-error   Stop on first failure (default: continue all)
    -p, --pattern GLOB    Filter filenames (e.g. -p '*Resistor*')
    -r, --reverse         Sort largest first (default: smallest first)
    --no-color            Disable colored output

${BOLD}ENVIRONMENT${RESET}
    ALTIUM_CLI            Path to altium-cli binary (default: altium-cli)

${BOLD}EXIT CODES${RESET}
    0  All files passed validation
    1  One or more files failed

${BOLD}EXAMPLES${RESET}
    # Run all fixtures
    ./scripts/test/validate_${FILE_TYPE_LOWER}.sh

    # Top 10 smallest files
    ./scripts/test/validate_${FILE_TYPE_LOWER}.sh --max 10

    # Specific files only
    ./scripts/test/validate_${FILE_TYPE_LOWER}.sh data/schlib/Foo.SchLib data/schlib/Bar.SchLib

    # Stop on first error, verbose
    ./scripts/test/validate_${FILE_TYPE_LOWER}.sh -s -v
EOF
}

_show_help_roundtrip() {
  cat <<EOF
${BOLD}roundtrip_${FILE_TYPE_LOWER}.sh${RESET} - Roundtrip test all ${FILE_TYPE} fixture files

${BOLD}USAGE${RESET}
    ./scripts/test/roundtrip_${FILE_TYPE_LOWER}.sh [OPTIONS] [FILE...]

${BOLD}DESCRIPTION${RESET}
    For each file: validates, saves via \`altium save-as\`, then runs
    \`altium cfb diff --semantic\` to check for serialization differences.
    Files that fail validation are skipped. Sorted by size (smallest first).

${BOLD}OPTIONS${RESET}
    -h, --help            Show this help and exit
    -n, --max N           Process only the first N files (after sorting)
    -v, --verbose         Show full semantic diff output for failures
    -s, --stop-on-error   Stop on first failure (default: continue all)
    -p, --pattern GLOB    Filter filenames (e.g. -p '*Resistor*')
    -r, --reverse         Sort largest first (default: smallest first)
    --no-color            Disable colored output
    --keep [DIR]          Keep roundtripped files (default: temp dir printed)

${BOLD}ENVIRONMENT${RESET}
    ALTIUM_CLI            Path to altium-cli binary (default: altium-cli)

${BOLD}EXIT CODES${RESET}
    0  All tested files roundtripped identically
    1  One or more files had save or diff failures

${BOLD}EXAMPLES${RESET}
    # Run all fixtures
    ./scripts/test/roundtrip_${FILE_TYPE_LOWER}.sh

    # Top 5 smallest, verbose diffs
    ./scripts/test/roundtrip_${FILE_TYPE_LOWER}.sh --max 5 -v

    # Keep outputs for manual inspection
    ./scripts/test/roundtrip_${FILE_TYPE_LOWER}.sh --keep /tmp/roundtrip-out

    # Stop on first error
    ./scripts/test/roundtrip_${FILE_TYPE_LOWER}.sh -s
EOF
}

# ── File discovery ────────────────────────────────────────────────────────────
# Populates FILES array sorted by size.
# Uses EXTRA_ARGS if non-empty, otherwise discovers from FIXTURE_DIR.
_discover_files() {
  local candidates=()

  if [ ${#EXTRA_ARGS[@]} -gt 0 ]; then
    # Explicit files from command line
    for f in "${EXTRA_ARGS[@]}"; do
      [ -f "$f" ] && candidates+=("$f")
    done
  else
    # Discover from fixture dir
    if [ -d "$FIXTURE_DIR" ]; then
      for f in "$FIXTURE_DIR"/*."$FILE_EXT"; do
        [ -f "$f" ] && candidates+=("$f")
      done
    fi
  fi

  # Filter by pattern if set
  if [ -n "$PATTERN" ]; then
    local filtered=()
    for f in "${candidates[@]}"; do
      local base
      base=$(basename "$f")
      # shellcheck disable=SC2254
      case "$base" in
        $PATTERN) filtered+=("$f") ;;
      esac
    done
    candidates=("${filtered[@]+"${filtered[@]}"}")
  fi

  # Sort by size (ascending = smallest first by default)
  local sort_flag=""
  $REVERSE && sort_flag="-r"

  FILES=()
  if [ ${#candidates[@]} -gt 0 ]; then
    while IFS= read -r line; do
      FILES+=("${line#* }")
    done < <(
      for f in "${candidates[@]}"; do
        stat --printf='%s %n\n' "$f" 2>/dev/null || stat -f '%z %N' "$f" 2>/dev/null
      done | sort -n $sort_flag
    )
  fi

  # Apply --max
  if [ "$MAX_FILES" -gt 0 ] && [ ${#FILES[@]} -gt "$MAX_FILES" ]; then
    FILES=("${FILES[@]:0:$MAX_FILES}")
  fi
}

# ── Progress ──────────────────────────────────────────────────────────────────
_file_size_human() {
  local bytes
  bytes=$(stat --printf='%s' "$1" 2>/dev/null || stat -f '%z' "$1" 2>/dev/null)
  if [ "$bytes" -ge 1048576 ]; then
    echo "$((bytes / 1048576))M"
  elif [ "$bytes" -ge 1024 ]; then
    echo "$((bytes / 1024))K"
  else
    echo "${bytes}B"
  fi
}

# ── Validate runner ───────────────────────────────────────────────────────────
run_validate() {
  _HELP_MODE="validate"
  _show_help() { _show_help_validate; }
  _parse_args "$@"
  _discover_files

  if [ ${#FILES[@]} -eq 0 ]; then
    echo "${YELLOW}No ${FILE_TYPE} files found.${RESET}"
    echo "Fixture directory: ${FIXTURE_DIR}/"
    echo "Make sure test fixtures are cloned (see CLAUDE.md)."
    exit 1
  fi

  local total=${#FILES[@]}
  local pass=0 fail=0 idx=0
  local fail_file
  fail_file=$(mktemp)
  trap "rm -f '$fail_file'" EXIT

  echo "${BOLD}Validating ${total} ${FILE_TYPE} files${RESET}"
  if [ "$MAX_FILES" -gt 0 ]; then
    echo "${DIM}(limited to ${MAX_FILES} files)${RESET}"
  fi
  echo ""

  for f in "${FILES[@]}"; do
    idx=$((idx + 1))
    local base size
    base=$(basename "$f")
    size=$(_file_size_human "$f")

    local out
    out=$($CLI validate "$f" 2>&1) && rc=0 || rc=$?

    if [ $rc -eq 0 ]; then
      pass=$((pass + 1))
      if $VERBOSE; then
        printf "[%3d/%d] ${GREEN}PASS${RESET} %-50s ${DIM}%s${RESET}\n" "$idx" "$total" "$base" "$size"
      fi
    else
      fail=$((fail + 1))
      local reason
      reason=$(echo "$out" | head -1)
      printf "[%3d/%d] ${RED}FAIL${RESET} %-50s ${DIM}%s${RESET}\n" "$idx" "$total" "$base" "$size"
      printf "        ${DIM}%s${RESET}\n" "$reason"
      echo "${base}|${reason}" >> "$fail_file"

      if $STOP_ON_ERROR; then
        echo ""
        echo "${RED}Stopping on first error (--stop-on-error).${RESET}"
        if $VERBOSE; then
          echo ""
          echo "$out"
        fi
        break
      fi
    fi
  done

  # ── Summary ───────────────────────────────────────────────────────────────
  echo ""
  echo "${BOLD}=== ${FILE_TYPE} Validation Summary ===${RESET}"
  echo "Total: ${total}  ${GREEN}Pass: ${pass}${RESET}  ${RED}Fail: ${fail}${RESET}"

  if [ -s "$fail_file" ] && ! $STOP_ON_ERROR; then
    echo ""
    echo "${BOLD}=== Failure Categories ===${RESET}"
    cut -d'|' -f2 "$fail_file" \
      | sed 's/block #[0-9]*/block #N/g; s/record #[0-9]*/record #N/g' \
      | sort | uniq -c | sort -rn
  fi

  [ "$fail" -eq 0 ]
}

# ── Roundtrip runner ──────────────────────────────────────────────────────────
run_roundtrip() {
  _HELP_MODE="roundtrip"
  _show_help() { _show_help_roundtrip; }
  _parse_args "$@"
  _discover_files

  if [ ${#FILES[@]} -eq 0 ]; then
    echo "${YELLOW}No ${FILE_TYPE} files found.${RESET}"
    echo "Fixture directory: ${FIXTURE_DIR}/"
    echo "Make sure test fixtures are cloned (see CLAUDE.md)."
    exit 1
  fi

  # Set up output directory
  local out_dir
  if [ "$KEEP_DIR" = "__auto__" ]; then
    KEEP_DIR=$(mktemp -d -t "roundtrip-${FILE_TYPE_LOWER}-XXXXXX")
    echo "${DIM}Keeping roundtripped files in: ${KEEP_DIR}${RESET}"
    out_dir="$KEEP_DIR"
  elif [ -n "$KEEP_DIR" ]; then
    mkdir -p "$KEEP_DIR"
    out_dir="$KEEP_DIR"
  else
    out_dir=$(mktemp -d)
    trap "rm -rf '$out_dir'" EXIT
  fi

  local total=${#FILES[@]}
  local pass=0 fail_save=0 fail_diff=0 skipped=0 idx=0
  local tested=0

  echo "${BOLD}Roundtrip testing ${total} ${FILE_TYPE} files${RESET}"
  if [ "$MAX_FILES" -gt 0 ]; then
    echo "${DIM}(limited to ${MAX_FILES} files)${RESET}"
  fi
  echo ""

  for f in "${FILES[@]}"; do
    idx=$((idx + 1))
    local base size
    base=$(basename "$f")
    size=$(_file_size_human "$f")

    # Skip files that don't validate
    if ! $CLI validate "$f" >/dev/null 2>&1; then
      skipped=$((skipped + 1))
      if $VERBOSE; then
        printf "[%3d/%d] ${YELLOW}SKIP${RESET} %-50s ${DIM}%s (validation failed)${RESET}\n" "$idx" "$total" "$base" "$size"
      fi
      continue
    fi

    tested=$((tested + 1))
    local out_file="$out_dir/$base"

    # Try save-as
    local save_out
    save_out=$($CLI save-as "$f" "$out_file" 2>&1) && rc=0 || rc=$?
    if [ $rc -ne 0 ]; then
      fail_save=$((fail_save + 1))
      printf "[%3d/%d] ${RED}SAVE-FAIL${RESET} %-44s ${DIM}%s${RESET}\n" "$idx" "$total" "$base" "$size"
      printf "         ${DIM}%s${RESET}\n" "$(echo "$save_out" | head -1)"

      if $STOP_ON_ERROR; then
        echo ""
        echo "${RED}Stopping on first error (--stop-on-error).${RESET}"
        if $VERBOSE; then echo ""; echo "$save_out"; fi
        break
      fi
      continue
    fi

    # Semantic diff
    local diff_out
    diff_out=$($CLI cfb diff --semantic --case-insensitive-keys "$f" "$out_file" 2>&1) && rc=0 || rc=$?
    if [ $rc -eq 0 ]; then
      pass=$((pass + 1))
      if $VERBOSE; then
        printf "[%3d/%d] ${GREEN}PASS${RESET} %-50s ${DIM}%s${RESET}\n" "$idx" "$total" "$base" "$size"
      fi
    else
      fail_diff=$((fail_diff + 1))
      local issue_count
      issue_count=$(echo "$diff_out" | grep -oP 'Total issues:\s*\K\d+' | head -1)
      issue_count="${issue_count:-?}"
      printf "[%3d/%d] ${RED}DIFF-FAIL${RESET} %-44s ${DIM}%s (%s issues)${RESET}\n" "$idx" "$total" "$base" "$size" "$issue_count"

      if $VERBOSE; then
        echo "$diff_out" | sed 's/^/         /'
        echo ""
      fi

      if $STOP_ON_ERROR; then
        echo ""
        echo "${RED}Stopping on first error (--stop-on-error).${RESET}"
        if $VERBOSE; then echo ""; echo "$diff_out"; fi
        break
      fi
    fi

    # Clean individual file if not keeping
    if [ -z "$KEEP_DIR" ]; then
      rm -f "$out_file"
    fi
  done

  # ── Summary ───────────────────────────────────────────────────────────────
  echo ""
  echo "${BOLD}=== ${FILE_TYPE} Roundtrip Summary ===${RESET}"
  echo "Total files:     ${total}"
  if [ "$skipped" -gt 0 ]; then
    echo "${YELLOW}Skipped (validation fail): ${skipped}${RESET}"
  fi
  echo "Tested:          ${tested}"
  echo "${GREEN}Roundtrip pass:  ${pass}${RESET}"
  if [ "$fail_save" -gt 0 ]; then
    echo "${RED}Save-as errors:  ${fail_save}${RESET}"
  else
    echo "Save-as errors:  0"
  fi
  if [ "$fail_diff" -gt 0 ]; then
    echo "${RED}Diff failures:   ${fail_diff}${RESET}"
  else
    echo "Diff failures:   0"
  fi

  if [ -n "$KEEP_DIR" ]; then
    echo ""
    echo "${DIM}Roundtripped files kept in: ${KEEP_DIR}${RESET}"
  fi

  [ "$fail_save" -eq 0 ] && [ "$fail_diff" -eq 0 ]
}
