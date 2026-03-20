#!/bin/sh
# =============================================================================
# Example: session_end hook
# =============================================================================
# Fires when an agent session ends (completed, cancelled, exit, or error).
#
# Input (JSON on stdin):
#   { "hook_event_name": "SessionEnd", "session_id": "...",
#     "cwd": "/path/to/project", "reason": "completed|cancelled|exit|error",
#     "transcript_path": "/path/to/transcript.json" }
#
# Environment variables:
#   FSPEC_PROJECT_DIR, FSPEC_SESSION_ID, FSPEC_HOOK_EVENT, FSPEC_TRANSCRIPT_PATH
#
# Output: session_end hooks are fire-and-forget. stdout/stderr are captured
# but only used for warning messages. No blocking or context injection.
# =============================================================================

PAYLOAD=$(cat)

SESSION_ID="$FSPEC_SESSION_ID"
PROJECT_DIR="$FSPEC_PROJECT_DIR"
REASON=$(echo "$PAYLOAD" | grep -o '"reason":"[^"]*"' | head -1 | cut -d'"' -f4)

# Log session end
LOG_FILE="${PROJECT_DIR}/.fspec/hooks.log"
mkdir -p "$(dirname "$LOG_FILE")"
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] session_end   | session=${SESSION_ID} | reason=${REASON}" >> "$LOG_FILE"

# Example: Post a summary to Slack (replace with your webhook URL)
# if command -v curl >/dev/null 2>&1; then
#   curl -s -X POST "https://hooks.slack.com/services/YOUR/WEBHOOK/URL" \
#     -H 'Content-Type: application/json' \
#     -d "{\"text\": \"Agent session ended: ${REASON} (session: ${SESSION_ID})\"}" \
#     >/dev/null 2>&1
# fi

exit 0
