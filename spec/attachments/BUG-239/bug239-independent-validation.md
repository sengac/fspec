# BUG-239 independent validation — 2026-10-22

Verdict: **NOT READY FOR ACCEPTANCE**. Keep BUG-239 in validating; the
current green scenario suite misses three reproducible policy failures.
No production changes were made during this review.

## Blocking findings

### P1: Streaming provider HTTP failures leak Debug-escaped conversation JSON

`rust/common/src/logging/privacy.rs:209` decodes only a complete JSON string,
not strings nested inside a Rust Debug representation. OpenAI streaming logs
`?error` at `rust/patches/rig-core/src/providers/openai/completion/streaming.rs:342`.
Its HTTP client constructs `InvalidStatusCodeWithMessage(status, body)` at
`rust/patches/rig-core/src/http_client/mod.rs:257` for failed requests.

Compiling the actual privacy module and passing the real rig HTTP error type
through `event_fields` with the default policy produces:

```text
SSE error error=InvalidStatusCodeWithMessage(400, "{\"error\":{\"message\":\"quota exceeded\"},\"messages\":[{\"role\":\"user\",\"content\":\"DEBUG_PROMPT_PRIVATE\"}]}")
```

The escaped object cannot be parsed by `privacy_json.rs:9`, so the conversation
marker survives. This shared policy feeds default files, stderr and broadcast.
The existing application-provider regression explicitly uses `.with_stream(false)`
and therefore does not cover this streaming failure record. Add a real streaming
HTTP-failure regression and separate actionable errors from response payloads.

### P1: Known provider instruction keys bypass default payload classification

`rust/common/src/logging/privacy.rs:83` recognizes `system_instructions` but
not `system` or `instructions`. A valid JSON diagnostic body containing
`{"error":{"message":"quota exceeded"},"system":"ANTHROPIC_SYSTEM_PRIVATE","instructions":"RESPONSES_SYSTEM_PRIVATE"}`
retains both payload markers under the default policy.

These are established provider fields, not arbitrary custom names: Anthropic
uses `system` (`providers/anthropic/completion.rs:833`) and OpenAI Responses
uses `instructions` (`providers/openai/responses_api/mod.rs:49`). Classify known
provider preamble keys consistently, including their opt-in size bounds.

### P2: Header redaction inside JSON deletes actionable root causes

`rust/common/src/logging/privacy.rs:154` serializes sanitized JSON before
running the header regex at line 120. Newlines within decoded JSON strings
become literal `\n` escapes. The regex recognizes only physical newline
boundaries and consumes the remainder of the serialized record.

For a JSON error message containing `transport failed\nAuthorization: Basic
dXNlcjpwYXNzd29yZA==\nroot cause: access denied`, followed by a `status: 401`
field, the compiled formatter produces only:

```text
provider failed: {"error":{"message":"transport failed\nAuthorization: [REDACTED]
```

Credentials disappear, but so do `root cause: access denied` and status 401.
Redact decoded string values before reserialization; test both privacy modes.

## Independently rerun checks

- Existing BUG-239 scenario tests: **6/6 PASS**. Command:
  `cargo test --profile ci-test -p codelet-fspec --test bug239_review_privacy --test bug239_privacy_logging -j 2 -- scenario_ --nocapture`.
  Output: `/tmp/bug239-independent-validation-tests.log`.
- Synthetic same-input comparison: 135903 to 1691 bytes; maximum record
  32617 to 302 bytes. Existing correlation/error-chain assertions pass.
- `cargo check --profile ci-test -p codelet-fspec --bin fspec -j 2`: PASS.
- `cargo clippy --profile ci-test -p codelet-common -p codelet-rpc -p codelet-tools -p codelet-cli -p codelet-agent-loop -p codelet-sessions -p codelet-fspec -j 2`: PASS,
  with the existing unrelated `fspec-tui/src/store/agent_view/token_state.rs:100`
  collapsible-match warning. Output: `/tmp/bug239-independent-quality.log`.
- RPC `log_layer_cycle_filter`: **2/2 PASS**; existing runtime tarpc-filter
  regression: **1/1 PASS**. Same quality log.
- Both BUG-239 feature files pass scoped syntax validation, tag validation,
  and coverage-file existence audits. `git diff --check`: PASS.
- Repository-wide tag validation separately reports 526 invalid feature files;
  both BUG-239 files pass individually. No unrelated tags were changed.
- Additional compiled policy probes: **3/3 FAIL**, reproducing the findings
  above. Source `/tmp/bug239_validation_probe.rs`; output
  `/tmp/bug239-independent-probes.log`. These temporary probes compile the real
  privacy module and use the real rig HTTP error type; they are not full HTTP
  integration tests and do not claim to be new scenario-linked acceptance tests.

All fixtures use synthetic markers. No live conversations or credentials were
copied into this evidence. No workspace-wide Cargo tests, commits or branches.
