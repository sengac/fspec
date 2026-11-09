# PROV-168 — evidence (from the user's report, 2026-11-01)

## Symptom
fspec TUI, session #1 on **Claude Opus 5.5** (Anthropic provider, Claude subscription / OAuth), prompt `what is 3 + 2`:

```
API Error: HttpError: Invalid status code 400 Bad Request with message:
{"type":"error","error":{"type":"invalid_request_error",
 "message":"Claude Code 2.1.3 does not support this model; version 2.1.280 or newer is required. Run 'claude update', or update the Claude desktop app, then try again.",
 "details":{"error_code":"claude_code_version_too_old"}},
 "request_id":"req_011CgbiMvtn6fZCpPtYjaUSt"}
```

The header showed `tokens: 0↓ 0↑ [context unknown]`. Screenshot: `claude-code-version-too-old.png`.

## What fspec sends today (code as of this card)
- `rust/providers/src/claude_oauth.rs:54` — `pub const CLAUDE_USER_AGENT: &str = "claude-cli/2.1.3 (external, cli)";`
- `rust/providers/src/claude.rs:299-309` (OAuth client) and `:398` — sets `User-Agent: CLAUDE_USER_AGENT` and `x-app: cli`.
- `rust/providers/src/claude.rs` `get_anthropic_beta_header_for_model` → `build_beta_headers(model, oauth)` — model-family rules documented only up to the 4.6 generation (adaptive thinking, context-1m, interleaved-thinking, `claude-code-20250219`).
- `get_system_prompt_prefix` → `CLAUDE_CODE_PROMPT_PREFIX` (required for OAuth).

## To investigate
1. The protocol the current Claude Code release uses for subscription OAuth requests: client version string, `User-Agent`, `x-app`, any other identifying headers, `anthropic-beta` values per model family, system prompt prefix, metadata fields.
2. Thinking parameters for current model families (adaptive thinking, effort levels) and any new required request fields.
3. How fspec should keep this current without a hard-coded stale version (no fallbacks): where the version and betas come from, and how a version-too-old rejection is reported to the user (clear, actionable error; the `[context unknown]` display for the failed turn).
