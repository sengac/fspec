#!/bin/sh
# =============================================================================
# Example: pre_tool_use hook
# =============================================================================
# Fires BEFORE a tool executes. Can Allow, Deny, or Ask for permission.
#
# Input (JSON on stdin):
#   { "hook_event_name": "PreToolUse", "session_id": "...",
#     "cwd": "/path/to/project", "tool_name": "Bash",
#     "tool_input": {"command": "..."}, "transcript_path": "..." }
#
# Environment variables:
#   FSPEC_PROJECT_DIR, FSPEC_SESSION_ID, FSPEC_HOOK_EVENT, FSPEC_TRANSCRIPT_PATH
#
# Output (Claude Code compatible JSON):
#   {"hookSpecificOutput":{"permissionDecision":"allow"}}  → auto-approve
#   {"hookSpecificOutput":{"permissionDecision":"deny","reason":"..."}} → block
#   {"hookSpecificOutput":{"permissionDecision":"ask"}}   → prompt user
#   Exit 0 with no JSON → Continue (no opinion, use default policy)
#   Exit 2 + stderr → Deny
#
# Short-circuit: Allow/Deny stops evaluation of remaining hook groups.
#                Continue passes to the next hook group.
# =============================================================================

PAYLOAD=$(cat)

# Log that this hook fired
PROJECT_DIR="$FSPEC_PROJECT_DIR"
LOG_FILE="${PROJECT_DIR}/.fspec/hooks.log"
mkdir -p "$(dirname "$LOG_FILE")"

# Extract tool_name and tool_input from the payload
TOOL_NAME=$(echo "$PAYLOAD" | grep -o '"tool_name":"[^"]*"' | head -1 | cut -d'"' -f4)
COMMAND_PREVIEW=$(echo "$PAYLOAD" | grep -o '"command":"[^"]*"' | head -1 | cut -d'"' -f4 | head -c 80)
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] pre_tool_use  | tool=${TOOL_NAME} | cmd=${COMMAND_PREVIEW}" >> "$LOG_FILE"

# This example script is configured with matcher "Bash" in fspec-hooks.json,
# so TOOL_NAME will always be "Bash" when this script runs. But we check
# anyway for defensive programming.

if [ "$TOOL_NAME" = "Bash" ]; then
  # Extract the command from tool_input
  # Using a simple grep approach — production scripts should use jq
  COMMAND=$(echo "$PAYLOAD" | grep -o '"command":"[^"]*"' | head -1 | cut -d'"' -f4)

  # Block destructive commands
  case "$COMMAND" in
    *"rm -rf /"*|*"rm -rf /*"*|*"mkfs"*|*"dd if="*|*"> /dev/sda"*)
      echo '{"hookSpecificOutput":{"permissionDecision":"deny"},"reason":"Destructive system command blocked by security hook"}' 
      exit 0
      ;;
    *"chmod 777"*|*"chmod -R 777"*)
      echo '{"hookSpecificOutput":{"permissionDecision":"deny"},"reason":"Insecure permissions change blocked"}' 
      exit 0
      ;;
    *"curl"*|*"wget"*)
      # Network commands require explicit user approval
      echo '{"hookSpecificOutput":{"permissionDecision":"ask"}}' 
      exit 0
      ;;
  esac
fi

# No opinion — let other hook groups or default policy decide
exit 0
