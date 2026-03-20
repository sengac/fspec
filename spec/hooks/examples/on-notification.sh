#!/bin/sh
# =============================================================================
# Example: notification hook
# =============================================================================
# Fires when the agent system emits a notification (e.g., permission prompts,
# task completion, errors).
#
# Input (JSON on stdin):
#   { "hook_event_name": "Notification", "session_id": "...",
#     "cwd": "/path/to/project", "notification_type": "permission_prompt",
#     "title": "Tool Permission", "message": "Allow Bash?",
#     "transcript_path": "..." }
#
# Environment variables:
#   FSPEC_PROJECT_DIR, FSPEC_SESSION_ID, FSPEC_HOOK_EVENT, FSPEC_TRANSCRIPT_PATH
#
# Output: notification hooks are fire-and-forget. stdout/stderr are captured
# but only used for warning messages.
# =============================================================================

PAYLOAD=$(cat)

SESSION_ID="$FSPEC_SESSION_ID"
PROJECT_DIR="$FSPEC_PROJECT_DIR"

# Extract notification fields
NOTIFICATION_TYPE=$(echo "$PAYLOAD" | grep -o '"notification_type":"[^"]*"' | head -1 | cut -d'"' -f4)
TITLE=$(echo "$PAYLOAD" | grep -o '"title":"[^"]*"' | head -1 | cut -d'"' -f4)
MESSAGE=$(echo "$PAYLOAD" | grep -o '"message":"[^"]*"' | head -1 | cut -d'"' -f4)

# Log notification
LOG_FILE="${PROJECT_DIR}/.fspec/hooks.log"
mkdir -p "$(dirname "$LOG_FILE")"
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] notification  | type=${NOTIFICATION_TYPE} | title=${TITLE} | message=${MESSAGE}" >> "$LOG_FILE"

# Example: Send desktop notification on macOS
# if command -v osascript >/dev/null 2>&1; then
#   osascript -e "display notification \"${MESSAGE}\" with title \"${TITLE}\""
# fi

exit 0
