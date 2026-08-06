#!/bin/sh
# =============================================================================
# Example: post_tool_use hook
# =============================================================================
# Fires AFTER a tool finishes executing. Can inject additional context
# (e.g., lint results) as system messages into the conversation.
#
# Input (JSON on stdin):
#   { "hook_event_name": "PostToolUse", "session_id": "...",
#     "cwd": "/path/to/project", "tool_name": "Write",
#     "tool_input": {"file_path": "...", "content": "..."},
#     "tool_response": "...", "transcript_path": "..." }
#
# Environment variables:
#   FSPEC_PROJECT_DIR, FSPEC_SESSION_ID, FSPEC_HOOK_EVENT, FSPEC_TRANSCRIPT_PATH
#
# Output:
#   - JSON {hookSpecificOutput:{additionalContext:"lint warnings..."}}
#     → injected as system context so agent can fix issues
#   - Exit 0 with no output → success, no context injection
#   - Timeout → warning logged, execution continues
# =============================================================================

PAYLOAD=$(cat)

PROJECT_DIR="$FSPEC_PROJECT_DIR"

# Log that this hook fired
LOG_FILE="${PROJECT_DIR}/.fspec/hooks.log"
mkdir -p "$(dirname "$LOG_FILE")"

# Extract the file path from tool_input
FILE_PATH=$(echo "$PAYLOAD" | grep -o '"file_path":"[^"]*"' | head -1 | cut -d'"' -f4)
TOOL_NAME=$(echo "$PAYLOAD" | grep -o '"tool_name":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] post_tool_use | tool=${TOOL_NAME} | file=${FILE_PATH}" >> "$LOG_FILE"

# Only lint TypeScript/JavaScript files
case "$FILE_PATH" in
  *.ts|*.tsx|*.js|*.jsx)
    # Run a linter if available and the file exists
    if [ -n "$FILE_PATH" ] && [ -f "$FILE_PATH" ]; then
      # Example: run eslint and capture output
      # LINT_OUTPUT=$(cd "$PROJECT_DIR" && npx eslint "$FILE_PATH" 2>&1 || true)
      # if [ -n "$LINT_OUTPUT" ]; then
      #   # Escape the output for JSON
      #   ESCAPED=$(echo "$LINT_OUTPUT" | head -20 | tr '\n' ' ' | sed 's/"/\\"/g')
      #   echo "{\"hookSpecificOutput\":{\"additionalContext\":\"Lint results for ${FILE_PATH}: ${ESCAPED}\"}}"
      # fi
      :
    fi
    ;;
esac

# Default: success with no context injection
exit 0
