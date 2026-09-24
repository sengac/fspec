#!/bin/zsh
# capture-profile.sh — capture raw profiling data from a running fspec process.
#
# Captures (raw, no processing — flamegraph conversion happens later):
#   <out>/sample.txt        — macOS `sample` call graph (all threads, all stacks)
#   <out>/threads-lldb.txt  — lldb thread list + full backtraces of every thread
#   <out>/meta.txt          — pid, path, timestamps, sample duration
#
# Usage:
#   ./capture-profile.sh <pid> <out-dir> [sample-seconds]
#
#   <pid>             PID of the fspec process to profile
#   <out-dir>         directory to write raw captures into (created if missing)
#   [sample-seconds]  how long to sample (default 30)
#
# Notes:
#   - Run as the user that owns the process.
#   - For a HANG, 60-120s gives cleaner parked-stack data; for CPU-bound
#     freezes 30s is plenty.
#   - The lldb step briefly stops the process (~1-2s) AFTER the sample
#     window, so it doesn't distort the sample data.
#   - meta.txt records the process CPU% over the sample window, which tells
#     us whether the freeze is a CPU spin (busy) or a blocked/parked thread.

set -uo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <pid> <out-dir> [sample-seconds]" >&2
  exit 2
fi

PID=$1
OUT=$2
SECS=${3:-30}

if ! kill -0 "$PID" 2>/dev/null; then
  echo "ERROR: process $PID is not running (or not owned by you)" >&2
  exit 1
fi

PROCPATH=$(ps -p "$PID" -o comm= 2>/dev/null)
echo "=== fspec raw profiling capture"
echo "=== pid: $PID  path: $PROCPATH"
echo "=== out dir: $OUT"
echo "=== sample window: ${SECS}s"
echo

mkdir -p "$OUT"

# --- 0. CPU% at start (for busy-vs-blocked diagnosis) ---------------------
CPU_BEFORE=$(ps -p "$PID" -o %cpu= | tr -d ' ')

# --- 1. sample (call graph, all threads) ----------------------------------
echo "-- [1/2] sampling for ${SECS}s (all threads, 1ms interval)..."
sample "$PID" "$SECS" 1 -file "$OUT/sample.txt"
SAMPLE_RC=$?
if [[ $SAMPLE_RC -ne 0 ]]; then
  echo "ERROR: sample exited with $SAMPLE_RC" >&2
  exit $SAMPLE_RC
fi
CPU_AFTER=$(ps -p "$PID" -o %cpu= | tr -d ' ')
echo "     -> $OUT/sample.txt ($(wc -c < "$OUT/sample.txt" | tr -d ' ') bytes)"

# --- 2. lldb thread backtraces (brief stop-the-world) ---------------------
echo "-- [2/2] lldb: thread list + full backtraces (brief pause)..."
LLDB_SCRIPT="$OUT/.lldb-cmds"
cat > "$LLDB_SCRIPT" <<'EOF'
thread list
thread backtrace -all -c 200
detach
quit
EOF
( timeout 120 lldb -p "$PID" -s "$LLDB_SCRIPT" 2>&1 ) > "$OUT/threads-lldb.txt"
LLDB_RC=$?
rm -f "$LLDB_SCRIPT"
if [[ $LLDB_RC -ne 0 ]]; then
  echo "WARNING: lldb exited with $LLDB_RC — thread backtraces may be incomplete" >&2
else
  echo "     -> $OUT/threads-lldb.txt ($(wc -l < "$OUT/threads-lldb.txt" | tr -d ' ') lines)"
fi

# --- 3. metadata ----------------------------------------------------------
{
  echo "pid: $PID"
  echo "command: $PROCPATH"
  echo "captured_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "sample_duration_secs: $SECS"
  echo "macos: $(sw_vers -productVersion 2>/dev/null)"
  echo "cpu_before_pct: $CPU_BEFORE"
  echo "cpu_after_pct: $CPU_AFTER"
  echo "note: raw captures; process later with scripts/sample2folded.py + FlameGraph"
} > "$OUT/meta.txt"

echo
echo "=== DONE — raw captures in: $OUT"
echo "    sample.txt        (call graph; CPU% over window: ${CPU_BEFORE}% -> ${CPU_AFTER}%)"
echo "    threads-lldb.txt  (per-thread backtraces — where threads block)"
echo "    meta.txt          (capture metadata)"
