# BUG-239: Opt-in payload tracing and concise default logs

Research date: 2026-10-22. HITL explicitly selected privacy-conscious default logging
with opt-in full payload diagnostics, rather than a size-only change. No external
disclosure or credential compromise has been established. Source references are snapshots.

## Context and impact

Agent spans store full instructions, prompts, and completions. Formatting nested
events repeats those fields, burying short errors behind large conversation payloads.
Tool-execution records also contain arguments/results, including session originals.
This creates local-disk and diagnostic-bundle exposure as well as observability noise.
Changing span formatting alone would not cover every payload-bearing emitter or sink.

## Measured evidence

Input: `~/.fspec/logs/fspec-combined.log.2026-10-21` and
`~/.fspec/logs/fspec-combined.log.2026-10-22`; records before
`2026-10-22T03:32:00Z`, excluding new review activity.

| Measure | Value |
|---|---|
| Records | 1,665 |
| Total bytes | 7,481,139 |
| Records containing prompt/instruction/completion span fields | 139 |
| Bytes occupied by those entire records | 4,763,081, approximately 64% of total |
| Largest record | 93,008 bytes |

The 4.76 MB figure measures entire payload-bearing lines, not the exact byte count of
prompt values alone. Tool-result payloads may add exposure outside this measurement.

- Oct 21 log `:240`, `:242`, `:244`, and `:246` bury SSE failures after repeated instructions.
- Oct 21 log `:783` and `:784` repeat conversation content in warning span context.
- INFO `execute_tool` records contain tool arguments/results; session-history results
  must be included in the policy audit without reproducing them here.

## Code and sink inventory

- `rust/patches/rig-core/src/agent/prompt_request/streaming.rs:480` creates an INFO span
  containing `gen_ai.system_instructions`, prompt, completion, and usage fields.
- `rust/patches/rig-core/src/agent/prompt_request/streaming.rs:497` records prompt text;
  `rust/patches/rig-core/src/agent/prompt_request/streaming.rs:579` has another instruction span.
- `rust/fspec/src/common.rs:226` configures combined-mode file formatting and broadcast;
  `rust/fspec/src/common.rs:242` configures daemon stderr/broadcast;
  `rust/fspec/src/common.rs:260` configures client file formatting.
- Audit remaining tool execution, provider request/response, error, broadcast, and
  telemetry paths before claiming that default logs are payload-free.

BUG-238 owns routine DeepSearch severity. BUG-232 through BUG-237 may need concise
correlated diagnostics, but that must not silently reintroduce full payload logging.
Conversation persistence is a different product capability from diagnostic logging;
do not remove required session storage as a logging fix.

## Expected outcomes

1. Default diagnostic sinks contain concise metadata, operation/session/request IDs,
   durations, counts, and actionable errors/chains, without full instruction, prompt,
   completion, or tool argument/result bodies.
2. Full payload tracing requires deliberate explicit opt-in, documented with its
   sensitivity and effective scope; it must not become enabled accidentally by an
   ordinary broad log-level setting.
3. Sensitive-data safeguards apply even when payload diagnostics are requested;
   define credential handling and test it rather than promising that arbitrary text
   can always be perfectly redacted.
4. Routine nested events do not repeatedly expand entire payload-bearing span fields.
5. Root causes remain readable; reducing volume must not merely truncate away the
   final actionable error or destroy useful correlation.
6. Opt-in diagnostics remain useful for intentional forensic work with clearly
   documented storage, permissions, sharing, retention, and cleanup expectations.

## Decisions to settle during specification

- Configuration surface, scope, default, and whether changing it requires a restart.
- Which sinks can receive opted-in payloads and whether broadcast/telemetry require
  independent authorization from local file logging.
- Credential redaction rules, payload size bounds, retention policy, and file permissions.
- Backward compatibility for deliberate diagnostic consumers without maintaining an
  implicit privacy-unsafe default.

These details are not settled by this attachment; use Example Mapping before implementation.

## Regression evidence to produce

Use synthetic unique markers for system text, user text, completion text, tool inputs,
tool results, and credentials. Exercise nested spans, tool execution, and error chains
through relevant sinks. Assert default absence of payload markers, retained correlation
and root causes, explicit opt-in behavior, and credential safeguards. Never use real
credentials or private conversations as fixtures.

Review existing logging-test patterns such as `rust/fspec/tests/log005_tarpc_quiet.rs:1`
and `rust/napi/tests/rust_log_filter_test.rs:1`. Scope new tests to the affected packages
and test targets; do not run an unscoped workspace test suite.

## Scope limits and completion

Do not suppress all logging, change conversation persistence, silently enable payloads
for remote sinks, or delete existing user logs during this fix. Completion requires a
documented opt-in/privacy contract, scenario-linked sink-level tests, scoped quality
checks, and a before/after synthetic size/readability comparison without private data.

## ACDD execution, 2026-10-22

- Read AGENTS.md, docs/TESTING.md, foundation, tags, this attachment and current
  multi-client/tracing and broadcast-recursion specifications before edits.
- Entered specifying, captured rules/examples, wrote and formatted/validated
  privacy-conscious-diagnostic-logging.feature, then entered testing.
- Runtime RED: `cargo test --profile ci-test -p codelet-fspec --test
  bug239_privacy_logging -j 2`, `/tmp/bug239-red-runtime.log`: four scenario
  failures before production edits. Default client/combined and daemon leaked
  SYSTEM_PRIVATE; explicit local diagnostics and broadcast leaked KEY_SECRET.
  Four pre-existing common module tests passed. Earlier test compilation errors
  were corrected before recording this behavioral RED.
- Tests exercise compiled startup functions in isolated subprocesses, real
  rig SpanCombinator input/output recording, nested rig-compatible agent/tool
  spans, the real bridge input emitter, RPC broadcast and real rotating files.
  Fixtures contain only synthetic markers, not incident conversations.

### Implemented contract and audit

- Shared privacy-aware field/event formatting is wired into native fspec
  combined/client files, daemon stderr, common Rust diagnostics, diagnostic
  `log.entry` capture, and RPC broadcast. Broadcast stores safe span metadata
  independently, so local payload authorization cannot flow into WebSocket logs.
- `FSPEC_LOG_PAYLOADS=1` at startup authorizes only local combined/client files.
  Unset, `true`, and ordinary `RUST_LOG=trace` do not authorize payloads.
  Payload values are credential-redacted, bounded to 4096 Unicode characters,
  and span payload updates are consumed once instead of repeated on descendants.
- Credential-named fields are always redacted, as are recognizable bearer tokens
  and credential assignments in text/JSON. Arbitrary prose/custom credential
  formats are not guaranteed secret-free; provider/tool errors need review before
  sharing. Actionable error chains are retained rather than size-truncated.
- New files use `~/.fspec/logs/private/`, enforced Unix directory mode 0700,
  daily rotation and five files per prefix. This is not a byte quota or an
  elapsed-time expiry. Historical logs elsewhere are neither deleted nor sanitized.
  Windows requires user-controlled private ACLs; external stderr collectors own
  their retention. See docs/PRIVACY_LOGGING.md and the README Security link.
- Audited rig streaming/non-streaming agent spans, SpanCombinator input/output,
  provider request/response trace messages, tool arguments/results, raw SSE error
  records, native fspec formatters, common diagnostics, and broadcast. Converted
  free-text payload emitters in the bridge, CLI tool handlers, agent input/hooks,
  non-streaming rig tools, CLI-chatbot input, Mira, and Anthropic SSE failures
  into separately classified fields. OpenAI/Gemini/OpenRouter SSE parse failures
  retain parse errors and byte counts without default raw bodies. Channel-send
  failures use Display, not Debug of the rejected result/HITL response.
- Rig still exposes telemetry fields to custom subscribers; protection is at
  fspec's sink boundary, not a claim that arbitrary third-party subscribers are
  safe. No telemetry exporter or legacy NAPI callback is installed by native
  fspec. Conversation persistence and explicit debug-capture product storage
  remain unchanged. BUG-238's DeepSearch severity files were not edited.

### Validation evidence

- Final four scenario tests GREEN: `/tmp/bug239-green-final.log`, command:
  `cargo test --profile ci-test -p codelet-fspec --test
  bug239_privacy_logging -j 2 -- scenario_ --nocapture`. Filtering selects exactly
  the four scenario tests, excluding unrelated tests in the imported startup module.
- Synthetic same-stimulus legacy formatter control versus protected client sink:
  **135903 → 1691 bytes** (98.76% reduction), longest record **32617 → 302 bytes**.
  Session/request/tool-call IDs, input token count, elapsed milliseconds and the
  complete `transport failed: connection reset: peer closed` chain survive.
- `cargo check --profile ci-test -p codelet-fspec --bin fspec -j 2` PASS,
  `/tmp/bug239-check.log`; transitive production crates and patched rig compile.
- `cargo clippy --profile ci-test -p codelet-common -p codelet-rpc -p
  codelet-tools -p codelet-cli -p codelet-agent-loop -p codelet-sessions -p
  codelet-fspec -j 2` PASS, `/tmp/bug239-clippy-affected.log`. Existing unrelated
  `collapsible_match` warning in fspec-tui token_state.rs:100 remains untouched.
- Scoped Gherkin syntax/tag validation and coverage audit pass. Coverage links
  are 1:1 for all four scenarios, including shared policy and startup wiring.
- Existing RPC cycle-filter regressions PASS (2/2): `cargo test --profile
  ci-test -p codelet-rpc --test log_layer_cycle_filter -j 2`, evidence in
  `/tmp/bug239-cycle-regression.log`.
- Existing tarpc severity runtime regression PASS (1/1): `cargo test --profile
  ci-test -p codelet-fspec --test log005_tarpc_quiet -j 2 --
  default_env_filter_suppresses_tarpc_info_keeps_warn_and_other_info`, evidence
  in `/tmp/bug239-tarpc-regression.log`. The companion source-text test is not run.
- Initial validation attempts encountered an in-progress BUG-236 dependency
  export and then local integration wiring compiler errors; both were resolved
  before final GREEN/check/clippy evidence. No unrelated production fixes,
  workspace-wide tests, commits, or branches were created.
- Handoff state: **validating**, with all scoped gates above passed, ready for
  supervisor acceptance. All new files, including generated coverage, are under
  300 lines. Shared filesystem free space is approximately 4.6 GiB after workers'
  builds; avoid introducing another Cargo profile or unbounded test compilation.

## Independent review follow-up

- Reopened specifying/testing before extending the feature with two scenarios:
  complete embedded credential headers, and application provider error bodies.
- Confirmed the review's Basic authorization gap and complete-cookie gap.
  Audited stream_loop's DEBUG error record, context-overflow INFO record,
  terminal provider ERROR record and its cause-chain ERROR records: they format
  provider error strings directly, allowing valid serialized conversation bodies
  to bypass structured-field privacy classification.
- New compiled RED before production edits: `/tmp/bug239-review-red-runtime-final.log`,
  `cargo test --profile ci-test -p codelet-fspec --test bug239_review_privacy -j 2
  -- scenario_ --nocapture`: 0 passed, 2 failed. Header scenario leaked the Basic
  base64 in default client mode; application scenario leaked APP_SYSTEM_PRIVATE.
  Fixture compilation errors were corrected before this behavioral RED.
- The application regression runs the actual public run_agent_stream function
  with a real local axum HTTP endpoint and real non-streaming OpenAI adapter,
  returning HTTP 400 with synthetic error/conversation JSON. It asserts the real
  codelet_cli::interactive::stream_loop target and terminal provider message are
  present, so it is not a source-string assertion or a copied logging macro.
- The lifecycle gate requires one feature per test file. The two already-specified
  review scenarios were moved unchanged into
  privacy-diagnostic-header-and-provider-errors.feature, retaining BUG-239 tags,
  before returning to testing and relinking their review test file. No production
  changes occurred during that specification-only reorganization.
- After that split, the same two runtime failures were observed again in
  `/tmp/bug239-review-red-split.log` before entering implementing.
- Implemented complete recognizable Authorization/Proxy-Authorization and
  Cookie/Set-Cookie header-value redaction, including Basic credentials, quoted
  cookie-pair values and trailing cookie pairs. This applies independently of
  local payload authorization; tests cover client/combined/daemon/common sinks
  with opt-in both enabled and disabled.
- Valid JSON embedded in diagnostic strings now uses the same credential and
  content-key classification as structured fields, retaining error message/code
  metadata and surrounding text. This closes the audited app-target provider
  error bypass without editing the concurrently owned stream_loop.rs. Opted-in
  payload values remain bounded/redacted. A small private helper handles this
  policy; no application execution/retry behavior or conversation persistence
  changes were required.
- Limits remain explicit: arbitrary prose, malformed JSON, unknown content keys,
  bare encoded secrets and private information echoed inside otherwise actionable
  error messages are not guaranteed detectable. docs/PRIVACY_LOGGING.md records
  that boundary instead of claiming generic perfect redaction.
- Both review scenarios plus the four original scenarios GREEN (6/6) in
  `/tmp/bug239-review-green.log`: `cargo test --profile ci-test -p codelet-fspec
  --test bug239_review_privacy --test bug239_privacy_logging -j 2 -- scenario_
  --nocapture`. Original size/readability evidence is unchanged (135903 → 1691
  bytes; longest record 32617 → 302 bytes). All new code files remain under 300 lines.
- Follow-up quality gates PASS: `cargo check --profile ci-test -p codelet-common
  -p codelet-fspec -j 2` (`/tmp/bug239-review-check.log`) and the corresponding
  `cargo clippy` command (`/tmp/bug239-review-clippy.log`). The same pre-existing
  fspec-tui token_state.rs:100 collapsible_match warning remains unrelated.
- Both privacy feature files pass syntax/tag validation and coverage audits;
  four original scenarios and two review scenarios each map 1:1 to tests, with
  implementation links. `git diff --check` passes. Final follow-up state is
  **validating**, ready for supervisor acceptance; no commits or branches.
