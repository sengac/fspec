#!/usr/bin/env bash
#
# run-alloy.sh — Headless runner for Alloy formal-verification models.
#
# Iterates every `check` and `run` command in the .als files under
#   codelet/core/spec/compaction/
# and reports a pass/fail summary.
#
# Semantics:
#   - For a `check` command: UNSAT  ⇒ ✅ PROVED (no counterexample exists in scope)
#                            SAT    ⇒ ❌ COUNTEREXAMPLE (assertion violated)
#   - For a `run`   command: SAT    ⇒ ✅ SAT (model has a satisfying instance)
#                            UNSAT  ⇒ ❌ UNSAT (model contradicts itself)
#
# Requires alloy-analyzer 6.x and JDK 17+.
#   brew install alloy-analyzer
#
# Usage:
#   scripts/run-alloy.sh                  # run all models
#   scripts/run-alloy.sh trimmer.als      # run a specific model
#   scripts/run-alloy.sh --verbose        # show full Alloy output per check

set -uo pipefail

# ───── Configuration ────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MODELS_DIR="$REPO_ROOT/codelet/core/spec/compaction"

# Find a JDK ≥ 17. Alloy 6.2 is built against class-file 61 (JDK 17+).
detect_java_home() {
    if [[ -n "${JAVA_HOME:-}" ]] && "$JAVA_HOME/bin/java" -version 2>&1 \
            | head -1 | grep -qE 'version "(1[7-9]|[2-9][0-9])'; then
        echo "$JAVA_HOME"
        return
    fi
    for cand in \
        /opt/homebrew/opt/openjdk \
        /opt/homebrew/opt/openjdk@25 \
        /opt/homebrew/opt/openjdk@21 \
        /opt/homebrew/opt/openjdk@17 \
        /usr/local/opt/openjdk \
        /usr/local/opt/openjdk@17 \
        /Library/Java/JavaVirtualMachines/openjdk-25.jdk/Contents/Home \
        /Library/Java/JavaVirtualMachines/openjdk-21.jdk/Contents/Home \
        /Library/Java/JavaVirtualMachines/openjdk-17.jdk/Contents/Home
    do
        local jh="$cand"
        [[ -d "$jh/libexec/openjdk.jdk/Contents/Home" ]] && jh="$jh/libexec/openjdk.jdk/Contents/Home"
        if [[ -x "$jh/bin/java" ]]; then
            if "$jh/bin/java" -version 2>&1 | head -1 | grep -qE 'version "(1[7-9]|[2-9][0-9])'; then
                echo "$jh"
                return
            fi
        fi
    done
    echo ""
}

JAVA_HOME="$(detect_java_home)"
if [[ -z "$JAVA_HOME" ]]; then
    echo "❌ Could not find a JDK 17+ installation."
    echo "   Install with: brew install openjdk"
    exit 2
fi
export JAVA_HOME

if ! command -v alloy >/dev/null 2>&1; then
    echo "❌ 'alloy' command not found on PATH."
    echo "   Install with: brew install alloy-analyzer"
    exit 2
fi

# Per-run output directory (alloy exec writes solution files here).
WORK_DIR="$(mktemp -d -t fspec-alloy.XXXXXX)"
trap 'rm -rf "$WORK_DIR"' EXIT

# ───── Argument parsing ─────────────────────────────────────────────────────

VERBOSE=0
TARGETS=()
for arg in "$@"; do
    case "$arg" in
        -v|--verbose) VERBOSE=1 ;;
        -h|--help)
            sed -n '3,/^$/p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) TARGETS+=("$arg") ;;
    esac
done

if [[ ${#TARGETS[@]} -eq 0 ]]; then
    while IFS= read -r -d '' f; do
        TARGETS+=("$f")
    done < <(find "$MODELS_DIR" -maxdepth 1 -name '*.als' -print0 | sort -z)
fi

# ───── Runner ───────────────────────────────────────────────────────────────

# Extract `check` and `run` command names from a .als file.
# Matches multi-line declarations like `check Foo for 5 but 10 steps`.
extract_commands() {
    local file="$1"
    grep -E '^[[:space:]]*(check|run)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*' "$file" \
        | sed -E 's/^[[:space:]]*(check|run)[[:space:]]+([A-Za-z_][A-Za-z0-9_]*).*/\1 \2/'
}

# Run a single command. Echoes a one-line result.
#   $1 = path to .als
#   $2 = "check" | "run"
#   $3 = command name
run_one() {
    local file="$1" kind="$2" name="$3"
    cd "$WORK_DIR"
    local log="$WORK_DIR/${name}.log"
    alloy --defaultLevel info exec -q -f -c "$name" "$file" \
        > "$log" 2>&1 < /dev/null

    # Result detection — last "UNSAT!" / "SAT!" line wins for the trace.
    local outcome="UNKNOWN"
    if grep -qE '^\[main\] INFO alloy -[[:space:]]+UNSAT!' "$log"; then
        outcome="UNSAT"
    elif grep -qE '^\[main\] INFO alloy -[[:space:]]+SAT!' "$log"; then
        outcome="SAT"
    fi

    local symbol="❓" verdict="$outcome"
    if [[ "$kind" == "check" ]]; then
        case "$outcome" in
            UNSAT) symbol="✅"; verdict="PROVED      (no counterexample in scope)" ;;
            SAT)   symbol="❌"; verdict="COUNTEREXAMPLE FOUND" ;;
            *)     symbol="⚠️ "; verdict="result-not-detected (see $log)" ;;
        esac
    else  # run
        case "$outcome" in
            SAT)   symbol="✅"; verdict="SAT          (instance found)" ;;
            UNSAT) symbol="❌"; verdict="UNSAT        (model is contradictory)" ;;
            *)     symbol="⚠️ "; verdict="result-not-detected (see $log)" ;;
        esac
    fi

    printf '  %s  %-10s  %-44s %s\n' "$symbol" "$kind" "$name" "$verdict"

    if [[ $VERBOSE -eq 1 ]]; then
        echo "      ── log tail ────────────────────────────────────────────────"
        tail -n 20 "$log" | sed 's/^/      /'
        echo "      ──────────────────────────────────────────────────────────"
    fi

    [[ "$outcome" == "UNSAT" && "$kind" == "check" ]] && return 0
    [[ "$outcome" == "SAT"   && "$kind" == "run"   ]] && return 0
    return 1
}

# ───── Main loop ────────────────────────────────────────────────────────────

echo "fspec — Alloy verification runner"
echo "  JAVA_HOME = $JAVA_HOME"
echo "  alloy     = $(command -v alloy)"
echo "  models    = $MODELS_DIR"
echo

total=0
passed=0
failed=0
unknown=0

for model in "${TARGETS[@]}"; do
    if [[ ! -f "$model" ]]; then
        # Allow short names relative to MODELS_DIR
        if [[ -f "$MODELS_DIR/$model" ]]; then
            model="$MODELS_DIR/$model"
        else
            echo "⚠️  Skipping (not found): $model"
            continue
        fi
    fi
    echo "── $(basename "$model") ────────────────────────────────────────────"

    while read -r kind name; do
        [[ -z "$kind" ]] && continue
        total=$((total + 1))
        if run_one "$model" "$kind" "$name"; then
            passed=$((passed + 1))
        else
            log="$WORK_DIR/${name}.log"
            if grep -qE 'UNSAT|SAT' "$log" 2>/dev/null; then
                failed=$((failed + 1))
            else
                unknown=$((unknown + 1))
            fi
        fi
    done < <(extract_commands "$model")
    echo
done

echo "────────────────────────────────────────────────────────────"
printf 'Total: %d   ✅ pass: %d   ❌ fail: %d   ⚠️  unknown: %d\n' \
    "$total" "$passed" "$failed" "$unknown"

[[ $failed -eq 0 && $unknown -eq 0 ]] && exit 0
exit 1
