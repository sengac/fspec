#!/bin/sh
# =============================================================================
# Example: user_prompt_submit hook
# =============================================================================
# Fires after the user submits a prompt, before the agent processes it.
# Can BLOCK the prompt (exit 2 + stderr, or JSON continue:false).
#
# Input (JSON on stdin):
#   { "hook_event_name": "UserPromptSubmit", "session_id": "...",
#     "cwd": "/path/to/project", "prompt": "the user's message",
#     "transcript_path": "/path/to/transcript.json" }
#
# Environment variables:
#   FSPEC_PROJECT_DIR, FSPEC_SESSION_ID, FSPEC_HOOK_EVENT, FSPEC_TRANSCRIPT_PATH
#
# Output:
#   - Exit 0 + plain text stdout → context injected, prompt allowed
#   - Exit 0 + JSON {continue:true, hookSpecificOutput:{additionalContext:"..."}}
#     → context injected, prompt allowed
#   - Exit 2 + stderr message → prompt BLOCKED, message shown to user
#   - JSON {continue:false, reason:"..."} → prompt BLOCKED
# =============================================================================

PAYLOAD=$(cat)

# Log that this hook fired
PROJECT_DIR="$FSPEC_PROJECT_DIR"
LOG_FILE="${PROJECT_DIR}/.fspec/hooks.log"
mkdir -p "$(dirname "$LOG_FILE")"

# Extract the prompt text from the JSON payload
PROMPT=$(echo "$PAYLOAD" | grep -o '"prompt":"[^"]*"' | head -1 | cut -d'"' -f4)
PROMPT_PREVIEW=$(echo "$PROMPT" | head -c 60)
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] user_prompt   | prompt=${PROMPT_PREVIEW}" >> "$LOG_FILE"

# Policy check: block prompts that ask the agent to ignore its instructions
case "$PROMPT" in
  *"ignore all previous instructions"*|*"ignore your instructions"*|*"forget your rules"*)
    echo "Policy violation: prompt attempts to override agent instructions" >&2
    exit 2
    ;;
  *"delete everything"*|*"rm -rf"*|*"drop database"*)
    echo "Safety violation: prompt contains destructive intent" >&2
    exit 2
    ;;
esac

# If the prompt passes policy checks, optionally inject context
# Exit 0 with no stdout = allow with no additional context
exit 0
