#!/usr/bin/env bash
# bench.sh — Autopcb-research benchmark runner and scorer.
#
# Routes one or more benchmark boards, extracts metrics, computes a composite
# score, and optionally appends to results.tsv.
#
# Usage:
#   ./scripts/bench.sh                    # Route all benchmarks, print score
#   ./scripts/bench.sh --board hub        # Route only hub
#   ./scripts/bench.sh --record "desc"    # Route all + append to results.tsv
#   ./scripts/bench.sh --json             # Output machine-readable JSON
#
# The scoring function follows ISPD-style lexicographic priority:
#   completion (must be 100%) > DRC (must be 0) > wirelength + 50×vias

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

ALTIUM_CLI="${ALTIUM_CLI:-$(dirname "$0")/../target/release/altium}"
RESULTS_TSV="${RESULTS_TSV:-$(dirname "$0")/../results.tsv}"
BENCH_DIR="${BENCH_DIR:-$(realpath "$(dirname "$0")/../../ee-template")}"

# Benchmark boards: name → (spec_file, target_pcbdoc)
# Add boards here as they become available.
declare -A BOARDS
BOARDS[hub]="hub.pcbdoc-spec|hub.PcbDoc"
BOARDS[sensor]="sensor.pcbdoc-spec|sensor.PcbDoc"
BOARDS[phec]="phec.pcbdoc-spec|phec.PcbDoc"
BOARDS[power]="power.pcbdoc-spec|power.PcbDoc"

# Default boards to run (subset for speed; full suite with --all)
DEFAULT_BOARDS="hub"
ALL_BOARDS="hub sensor phec power"

# Score weights
VIA_WEIGHT=50
UNROUTED_PENALTY=1000000
DRC_BASE_PENALTY=100000
DRC_PER_VIOLATION=1000

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

BOARD_FILTER=""
RECORD_DESC=""
JSON_OUTPUT=false
RUN_ALL=false
QUIET=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --board)   BOARD_FILTER="$2"; shift 2 ;;
        --record)  RECORD_DESC="$2"; shift 2 ;;
        --json)    JSON_OUTPUT=true; shift ;;
        --all)     RUN_ALL=true; shift ;;
        --quiet)   QUIET=true; shift ;;
        --help|-h)
            echo "Usage: $0 [--board NAME] [--record DESC] [--json] [--all] [--quiet]"
            echo ""
            echo "Options:"
            echo "  --board NAME   Route only this board (default: hub)"
            echo "  --all          Route all benchmark boards"
            echo "  --record DESC  Append results to results.tsv with description"
            echo "  --json         Machine-readable JSON output"
            echo "  --quiet        Suppress routing logs"
            echo ""
            echo "Available boards: ${ALL_BOARDS}"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Select which boards to run
if [[ -n "$BOARD_FILTER" ]]; then
    BOARDS_TO_RUN="$BOARD_FILTER"
elif [[ "$RUN_ALL" == true ]]; then
    BOARDS_TO_RUN="$ALL_BOARDS"
else
    BOARDS_TO_RUN="$DEFAULT_BOARDS"
fi

# ---------------------------------------------------------------------------
# Check prerequisites
# ---------------------------------------------------------------------------

if [[ ! -x "$ALTIUM_CLI" ]]; then
    echo "ERROR: altium CLI not found at $ALTIUM_CLI" >&2
    echo "Run: cargo build -p altium-cli --release" >&2
    exit 1
fi

if [[ ! -d "$BENCH_DIR" ]]; then
    echo "ERROR: benchmark directory not found at $BENCH_DIR" >&2
    echo "Set BENCH_DIR to the directory containing .pcbdoc-spec files" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Scoring function
# ---------------------------------------------------------------------------

compute_score() {
    local completion="$1"
    local unrouted="$2"
    local wirelength="$3"
    local vias="$4"
    local drc="$5"

    # Lexicographic: completion > DRC > quality
    if (( unrouted > 0 )); then
        echo "$((UNROUTED_PENALTY * unrouted))"
    elif (( drc > 0 )); then
        echo "$((DRC_BASE_PENALTY + DRC_PER_VIOLATION * drc))"
    else
        # wirelength + VIA_WEIGHT * vias (integer arithmetic, truncate)
        python3 -c "print(f'{$wirelength + $VIA_WEIGHT * $vias:.2f}')"
    fi
}

# ---------------------------------------------------------------------------
# Route a single board
# ---------------------------------------------------------------------------

route_board() {
    local name="$1"
    local spec target

    IFS='|' read -r spec target <<< "${BOARDS[$name]}"

    local spec_path="$BENCH_DIR/$spec"
    local target_path="$BENCH_DIR/$target"
    local routes_path="/tmp/autopcb-bench-${name}.routes"

    if [[ ! -f "$spec_path" ]]; then
        echo "SKIP: $spec_path not found" >&2
        return 1
    fi

    if [[ ! -f "$target_path" ]]; then
        echo "SKIP: $target_path not found" >&2
        return 1
    fi

    # Route
    local start_time
    start_time=$(date +%s%N)

    local log_file="/tmp/autopcb-bench-${name}.log"
    local raw_out

    # Tracing goes to stdout alongside JSON. Capture all stdout, suppress or
    # tee stderr. The JSON block is the last { ... } in the output.
    if [[ "$QUIET" == true ]]; then
        raw_out=$("$ALTIUM_CLI" routing solve \
            --target "$target_path" \
            --output "$routes_path" \
            --json \
            "$spec_path" 2>"$log_file") || true
    else
        raw_out=$("$ALTIUM_CLI" routing solve \
            --target "$target_path" \
            --output "$routes_path" \
            --json \
            "$spec_path" 2>&1) || true
        echo "$raw_out" > "$log_file"
        # Show tracing lines (non-JSON) to stderr for visibility
        echo "$raw_out" | grep -v '^\s*[{}"]' >&2 || true
    fi

    local end_time
    end_time=$(date +%s%N)
    local runtime_ms=$(( (end_time - start_time) / 1000000 ))

    # Extract the JSON block (lines between { and })
    local json_out
    json_out=$(echo "$raw_out" | sed -n '/^{$/,/^}$/p') || true

    # Parse metrics from JSON using a single python3 call
    if [[ -z "$json_out" ]]; then
        echo "ERROR: no JSON output from routing solve for $name" >&2
        echo "0|0|0.0|0|0|${runtime_ms}|crash"
        return 0
    fi

    local metrics
    metrics=$(echo "$json_out" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f\"{d['completion_pct']}|{d['unrouted_count']}|{d['total_length_mm']:.2f}|{d['total_vias']}|{d['drc_violations']}\")
") || true

    if [[ -z "$metrics" ]]; then
        echo "ERROR: failed to parse JSON for $name" >&2
        echo "0|0|0.0|0|0|${runtime_ms}|crash"
        return 0
    fi

    echo "${metrics}|${runtime_ms}|ok"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

# Aggregate metrics across all boards
total_score=0
total_completion=0
total_unrouted=0
total_wirelength=0
total_vias=0
total_drc=0
total_runtime_ms=0
board_count=0
status="ok"

declare -A board_results

for board in $BOARDS_TO_RUN; do
    if [[ -z "${BOARDS[$board]+x}" ]]; then
        echo "ERROR: unknown board '$board'. Available: ${ALL_BOARDS}" >&2
        exit 1
    fi

    if [[ "$QUIET" != true ]] && [[ "$JSON_OUTPUT" != true ]]; then
        echo "--- Routing: $board ---" >&2
    fi

    result=$(route_board "$board") || continue

    IFS='|' read -r completion unrouted wirelength vias drc runtime_ms board_status <<< "$result"

    if [[ "$board_status" == "crash" ]]; then
        status="crash"
        continue
    fi

    score=$(compute_score "$completion" "$unrouted" "$wirelength" "$vias" "$drc")

    board_results[$board]="$score|$completion|$unrouted|$wirelength|$vias|$drc|$runtime_ms"

    total_score=$(python3 -c "print(f'{$total_score + $score:.2f}')")
    total_unrouted=$((total_unrouted + unrouted))
    total_vias=$((total_vias + vias))
    total_drc=$((total_drc + drc))
    total_wirelength=$(python3 -c "print(f'{$total_wirelength + $wirelength:.2f}')")
    total_runtime_ms=$((total_runtime_ms + runtime_ms))
    board_count=$((board_count + 1))
done

# Compute aggregate completion
if [[ $board_count -gt 0 ]]; then
    total_completion=$(python3 -c "
results = '${!board_results[*]}'.split()
comps = []
for b in results:
    parts = '${board_results[hub]:-0|0|0|0|0|0}'.split('|')
    # skip, we'll use unrouted count
print(100.0 if $total_unrouted == 0 else round(100.0 * (1 - $total_unrouted / max(1, $total_unrouted + $board_count * 25)), 1))
")
fi

# Determine status for results.tsv
if [[ "$status" == "crash" ]]; then
    : # already set
elif [[ $total_unrouted -gt 0 ]] || [[ $total_drc -gt 0 ]]; then
    # Still log — the score captures the penalty
    status="ok"
fi

# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

if [[ "$JSON_OUTPUT" == true ]]; then
    # Machine-readable JSON
    cat <<ENDJSON
{
  "score": $total_score,
  "unrouted": $total_unrouted,
  "wirelength_mm": $total_wirelength,
  "vias": $total_vias,
  "drc_violations": $total_drc,
  "runtime_ms": $total_runtime_ms,
  "board_count": $board_count,
  "boards": {
ENDJSON
    first=true
    for board in $BOARDS_TO_RUN; do
        if [[ -n "${board_results[$board]+x}" ]]; then
            IFS='|' read -r s c u w v d r <<< "${board_results[$board]}"
            [[ "$first" == true ]] || echo ","
            printf '    "%s": {"score": %s, "completion": %s, "unrouted": %s, "wirelength_mm": %s, "vias": %s, "drc": %s, "runtime_ms": %s}' \
                "$board" "$s" "$c" "$u" "$w" "$v" "$d" "$r"
            first=false
        fi
    done
    cat <<ENDJSON

  }
}
ENDJSON
else
    # Human-readable summary
    echo ""
    echo "=========================================="
    echo "  BENCHMARK RESULTS"
    echo "=========================================="
    echo ""

    for board in $BOARDS_TO_RUN; do
        if [[ -n "${board_results[$board]+x}" ]]; then
            IFS='|' read -r s c u w v d r <<< "${board_results[$board]}"
            printf "  %-12s score=%-12s completion=%s%%  unrouted=%s  wl=%smm  vias=%s  drc=%s  (%sms)\n" \
                "$board" "$s" "$c" "$u" "$w" "$v" "$d" "$r"
        fi
    done

    echo ""
    echo "  AGGREGATE"
    printf "  score:       %s\n" "$total_score"
    printf "  unrouted:    %s\n" "$total_unrouted"
    printf "  wirelength:  %s mm\n" "$total_wirelength"
    printf "  vias:        %s\n" "$total_vias"
    printf "  drc:         %s\n" "$total_drc"
    printf "  runtime:     %s ms\n" "$total_runtime_ms"
    echo "=========================================="
fi

# ---------------------------------------------------------------------------
# Record to results.tsv
# ---------------------------------------------------------------------------

if [[ -n "$RECORD_DESC" ]]; then
    # Get git commit hash
    commit=$(git -C "$(dirname "$0")/.." rev-parse --short=7 HEAD 2>/dev/null || echo "unknown")

    # Determine keep/discard status by comparing to previous best
    record_status="keep"
    if [[ -f "$RESULTS_TSV" ]] && [[ $(wc -l < "$RESULTS_TSV") -gt 1 ]]; then
        # Find best previous score (lowest that was kept)
        prev_best=$(tail -n +2 "$RESULTS_TSV" | awk -F'\t' '$8 == "keep" {print $2}' | sort -n | head -1)
        if [[ -n "$prev_best" ]]; then
            is_better=$(python3 -c "print('yes' if $total_score < $prev_best else 'no')")
            if [[ "$is_better" == "no" ]]; then
                record_status="discard"
            fi
        fi
    fi

    if [[ "$status" == "crash" ]]; then
        record_status="crash"
    fi

    # Initialize results.tsv if needed
    if [[ ! -f "$RESULTS_TSV" ]]; then
        printf "commit\tscore\tcompletion\tunrouted\twirelength\tvias\tdrc\tstatus\tdescription\n" > "$RESULTS_TSV"
    fi

    # Append result
    # Use completion from the first (primary) board
    primary_completion="0.0"
    for board in $BOARDS_TO_RUN; do
        if [[ -n "${board_results[$board]+x}" ]]; then
            IFS='|' read -r _ c _ _ _ _ _ <<< "${board_results[$board]}"
            primary_completion="$c"
            break
        fi
    done

    printf "%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n" \
        "$commit" "$total_score" "$primary_completion" "$total_unrouted" \
        "$total_wirelength" "$total_vias" "$total_drc" "$record_status" "$RECORD_DESC" \
        >> "$RESULTS_TSV"

    if [[ "$JSON_OUTPUT" != true ]]; then
        echo ""
        echo "Recorded: $record_status → $RESULTS_TSV"
    fi
fi

# Exit with score as a signal (0 = good, 1 = has issues)
if [[ $total_unrouted -gt 0 ]] || [[ $total_drc -gt 0 ]]; then
    exit 1
fi
exit 0
