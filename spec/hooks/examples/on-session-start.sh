#!/bin/sh
# =============================================================================
# Example: session_start hook
# =============================================================================
# Fires when an agent session starts (startup or resume).
#
# Input (JSON on stdin):
#   { "hook_event_name": "SessionStart", "session_id": "...",
#     "cwd": "/path/to/project", "source": "startup|resume",
#     "transcript_path": "/path/to/transcript.json" }
#
# Environment variables:
#   FSPEC_PROJECT_DIR      — workspace root path
#   FSPEC_SESSION_ID       — UUID of the agent session
#   FSPEC_HOOK_EVENT       — "SessionStart"
#   FSPEC_TRANSCRIPT_PATH  — path to the transcript file
#
# Output options:
#   - Plain text on stdout → injected as system-level context
#   - JSON on stdout with hookSpecificOutput.additionalContext → injected
#   - Exit 0 → success
#   - Exit 2 + stderr → warning (session_start doesn't block)
# =============================================================================

# Read the JSON payload from stdin
PAYLOAD=$(cat)

# Extract fields using basic shell tools (or use jq if available)
SESSION_ID="$FSPEC_SESSION_ID"
PROJECT_DIR="$FSPEC_PROJECT_DIR"
SOURCE=$(echo "$PAYLOAD" | grep -o '"source":"[^"]*"' | head -1 | cut -d'"' -f4)

# Log session start to a project-local hooks log
LOG_FILE="${PROJECT_DIR}/.fspec/hooks.log"
mkdir -p "$(dirname "$LOG_FILE")"
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] session_start | session=${SESSION_ID} | source=${SOURCE}" >> "$LOG_FILE"

# Inject additional context as a system message.
# Plain text stdout is automatically captured as additional context.
echo "Project coding standards: Use TypeScript with strict mode. All functions must have JSDoc comments. No console.log in production code."
