# Epic Review: RPC-066 — Cross-frontend integration test against stub provider

**Date:** 2026-05-25T12:15:00Z
**Reviewer:** Claude Code (fspec review skill)
**Scope:** RPC-066 only (no parent / siblings traversed — user requested focused review of this card)
**Card status at review start:** done
**Estimate:** 8 points
**Parent:** RPC-030 · **Depends on:** RPC-065 · **Blocks:** RPC-069

---

## Summary

- 🔴 **Critical:** 5 issues
- 🟡 **Warnings:** 6 issues
- 🟢 **Observations:** 3 items

The card delivers **the structural scaffolding** for a cross-frontend
integration test (StubProvider LlmProvider impl, register_stub_provider,
test-stub-provider cargo feature, normalisation pipeline, README docs).
However, the **end-to-end integration tests themselves do not actually
run successfully** when invoked with `--features test-stub-provider
-- --include-ignored`. Three of the four `#[ignore]`'d integration tests
fail. The remaining "tests" are source-shape regex assertions against
the test file's own body, which are tautological and do not prove the
described behaviour.

Per architecture note [I], the card author explicitly acknowledged the
risk that the agent-loop wiring may surface blocking bugs and authorised
splitting them into sibling cards rather than fixing inline. The
present review honours that boundary: blocking bugs are **flagged but
not fixed** in this card. Documentation honesty fixes that are clearly
in-scope (incorrect file docstring, incorrect sibling-card references)
**are applied**.

---

## Build & Test Verification

```
cargo build -p codelet-fspec                                              → OK
cargo build -p codelet-fspec --features test-stub-provider                → OK
cargo test -p codelet-fspec --test cross_frontend_parity                  → 7 pass, 4 ignored
cargo test --features test-stub-provider -p codelet-fspec
            --test cross_frontend_parity -- --include-ignored             → 8 pass, 3 FAIL
```

### Test failures with `--features test-stub-provider --include-ignored`

| Test | Outcome | Failure |
|------|---------|---------|
| `scenario_send_input_hello_yields_canned_stream` | ❌ FAIL | `create_session(None) ... the request exceeded its deadline` (line 146) |
| `scenario_scripted_run_matches_golden` | ❌ FAIL | golden fixture missing at `codelet/fspec/tests/fixtures/cross_frontend_run.jsonl` |
| `scenario_deny_network_egress_still_yields_canned_chunks` | ❌ FAIL | `create_session: the request exceeded its deadline` (line 638) |
| `scenario_fspec_daemon_boots_and_emits_a_port` | ✅ PASS | (daemon boot only — does not exercise sessions) |

---

## 🔴 Critical Issues (Must Fix)

### C1. The golden fixture file is missing — Rule [10] / Architecture note [A]#2 unmet

`codelet/fspec/tests/fixtures/cross_frontend_run.jsonl` is listed as a
hard deliverable:

- Rule [10]: *"Ship a Rust-pinned golden file at
  `codelet/fspec/tests/fixtures/cross_frontend_run.jsonl` as the
  initial regression baseline."*
- Architecture note [A]: *"(2) NEW
  codelet/fspec/tests/fixtures/cross_frontend_run.jsonl — Rust-pinned
  golden chunk stream."*
- Example [4]: *"…the captured chunk stream — after normalisation — is
  byte-identical to the golden file at
  codelet/fspec/tests/fixtures/cross_frontend_run.jsonl"*

The `tests/fixtures/` directory does not exist. `scenario_scripted_run_matches_golden`
fails fast on the missing-fixture check (line 371). The card cannot
discharge rule [10] without this file — but the regeneration path
requires the agent loop to actually produce chunks (see C2), so
generating it in this card is blocked.

**Resolution:** This is a sibling-card concern per architecture
note [I]. Flag clearly in card status.

---

### C2. The agent loop is not wired — `FspecSessionManagerHooks::spawn_agent_loop` is a no-op

`codelet/fspec/src/session_hooks.rs:29-35`:

```rust
fn spawn_agent_loop(
    &self,
    _session: Arc<BackgroundSession>,
    _input_rx: mpsc::Receiver<PromptInput>,
    _mcp_injection_rx: mpsc::Receiver<McpInjection>,
) {}
```

Rule [11] states: *"…register under slug `stub` via the existing
custom-provider mechanism so SessionManager::create_session('stub/canned',
...) reaches it via ProviderType::Custom. **This keeps the agent loop
in the test path.**"*

The whole premise of the card — *"drives every slash command end-to-end
against the real `fspec daemon` binary backed by a deterministic stub
`LlmProvider`"* — requires `spawn_agent_loop` to actually drive a loop.
It does not. Consequently, `send_input("hello")` is enqueued onto the
session's `input_tx` channel but **never consumed by anything**, and no
chunks are ever emitted. The canned `Text { text: "hi back" }, Done`
stream cannot be produced.

**Resolution:** Sibling-card territory per architecture note [I]. Must
not be fixed in RPC-066.

---

### C3. `ProviderManager` match arm for `ProviderType::Custom` does not consult the in-memory stub registry — Architecture note [J] only half-implemented

Architecture note [J]: *"…The ProviderManager's match arm for
ProviderType::Custom is **also extended** to look up the in-memory map
first, falling back to the existing RhaiCustomProvider path."*

The work landed half of this note:

- ✅ `codelet_providers::stub_provider::is_stub_registered(slug)` exists
- ✅ `codelet_providers::custom_provider_registered(slug)` consults the
  in-memory map (manager.rs:138)
- ✅ `codelet_providers::stub_provider::get_stub_provider(slug)` returns
  `Option<Arc<dyn LlmProvider>>` and is **publicly exported**
- ❌ `ProviderManager` never calls `get_stub_provider` — it has no
  in-memory map lookup in any get_*/match arm
- ❌ The downstream agent-loop code path that would actually need the
  `Arc<dyn LlmProvider>` (or its rig agent equivalent) for
  `ProviderType::Custom("stub")` still resolves through
  `RhaiCustomProvider`, which expects a disk-resident JSON config.

This is why `create_session` against `stub/canned` cannot produce live
chunks even before the C2 no-op `spawn_agent_loop` matters: the manager
has no path from `ProviderType::Custom("stub")` → `StubProvider` LlmProvider
instance.

**Resolution:** Sibling-card territory per architecture note [I].

---

### C4. Architecture note [B] tool-call branch was silently dropped — `complete_with_tools` for "trigger-tool" is degraded to plain text

Architecture note [B]: *"complete_with_tools depends on the scripted
run — for inputs containing 'trigger-tool' it emits a CompletionResponse
with content=MessageContent::ToolUse{...} so the SessionManager's tool
dispatcher exercises the ToolCall + ToolResult chunk path"*

`codelet/providers/src/stub_provider.rs:80-101` instead returns
`MessageContent::Text("hi back")` for the trigger-tool prompt:

```rust
// Note: the original architecture note [B] called for a
// MessageContent::ToolUse return shape. … this card covers …
// the tool-call branch is intentionally a text completion in this
// initial card; the follow-up sibling card (RPC-067/068) will add
// the ToolUse return and the noop_tool registration.
```

This affects scenario coverage materially:

- Scripted run step 4 (`send_input("trigger-tool")`) is supposed to emit
  `ToolCall` + `ToolResult` chunks per example [4].
- The normalisation pipeline tests `tool_call_id` substitution in the
  unit test `scenario_normalise_chunk_stream_substitutes_volatile_fields`,
  but the **end-to-end scripted run** never actually emits a `ToolCall`
  chunk through the real agent loop.
- The "RPC-067/068" sibling-card reference in the source comment is
  **factually wrong** — RPC-067 is dependency-rule regression tests,
  RPC-068 is the final TS-frontend regression audit. Neither addresses
  ToolUse/noop_tool wiring.

**Resolution:** Comment is being corrected to remove the false RPC-067/068
reference. The actual ToolUse wiring remains sibling-card territory.

---

### C5. The test file docstring lists `register_noop_tool()` as a deliverable that does not exist

`codelet/fspec/tests/cross_frontend_parity.rs:17`:

```
//!   - `codelet_providers::stub_provider::register_noop_tool()` (new)
```

Architecture note [L]: *"register a stub tool slug `noop_tool` in
codelet/providers/src/stub_provider.rs … behind test-support that
takes empty input and returns a static `"ok"` ToolResult. Wired
alongside register_stub_provider(). Without this the agent loop will
error out trying to find an unregistered tool."*

`grep register_noop_tool` against the entire codebase only finds the
docstring's claim. The function does not exist. Architecture note [L]
is unmet.

**Resolution:** Without the ToolUse path (C4) and the agent loop (C2),
`noop_tool` registration would be cosmetic dead code in this card. The
docstring is being corrected to remove the false deliverable claim;
actual noop_tool wiring is sibling-card territory.

---

## 🟡 Warnings (Should Fix)

### W1. Multiple "tests" are source-shape regex checks against the test file itself (tautological)

Four scenarios use `read_to_string(file!())` and `body.contains(...)`
to assert on the test file's own source code:

| Test | Asserts |
|------|---------|
| `scenario_regenerate_env_var_recorded_in_test_source` | The test source contains `"FSPEC_RPC_066_REGENERATE"` and `"cross_frontend_run.jsonl"` |
| `scenario_missing_fixture_fails_with_clear_hint` | The test source (stripped of comments) contains `"FSPEC_RPC_066_REGENERATE=1"` |
| `scenario_regression_catch_documented` | The test source contains the literal `"assert_eq!(actual, expected"` |
| `scenario_runtime_budget_is_enforced` | The test source contains `"Duration::from_secs(45)"` and `"tokio::time::timeout"` |

These pass by construction — as long as the assertion text is mentioned
anywhere in the file (including in the assertion itself!), the test
passes. They do **not** prove the described behaviour:

- W1a. The regenerate-env-var test proves nothing about whether the
  regenerate codepath actually writes the file when invoked.
- W1b. The missing-fixture-fails test proves nothing about whether the
  test actually emits the hint text on failure — it just looks for the
  literal in any context.
- W1c. The regression-catch test proves the assert_eq call exists; it
  does **not** demonstrate the test would catch the documented
  regression (because the scripted run can't run, per C2).
- W1d. The runtime-budget test proves the timeout call exists; it does
  **not** measure actual wall-clock time.

**Resolution:** Documenting; not removing — these are explicit choices
the card author made (with the agent loop unwired, behavioural
assertions are impossible). The shape-only nature is being acknowledged
in the test source comments.

---

### W2. `scenario_workspace_registers_the_stub_provider` is also a source-shape check

Asserts `common.rs` contains `"test-stub-provider"` and
`"register_stub_provider"` substrings, and `stub_provider.rs` contains
`"Once"`. None of this exercises the runtime registration. A real
behavioural test would dev-depend on `codelet-providers` with
`test-support` and call `register_stub_provider()` + `is_stub_registered("stub")`
directly. Adding the dev-dep would not be scope creep — `codelet-providers/test-support`
already exists.

**Resolution:** Leaving as-is. Adding a new dev-dep arrow is
borderline-scope and the runtime behaviour is supposed to be exercised
by the `#[ignore]`'d integration scenarios anyway (which currently fail
for unrelated reasons — C2/C3).

---

### W3. `scenario_deny_network_egress_still_yields_canned_chunks` does not verify network was denied

The scenario assertion `"And no reqwest::Client or eventsource-stream
code path fires during the run"` (feature file line 162) is not
exercised. The test only asserts that `Text + Done` arrive within 5s
under a dead proxy. With no live tracing assertion or code-path
detection, the negative claim is unsupported.

Compounding this, the test fails before reaching the assertion because
`create_session` times out (C2/C3).

**Resolution:** Sibling-card territory — requires either a tracing
subscriber that watches for `reqwest`/`eventsource` spans, or a
build-time assertion. Documenting only.

---

### W4. Coverage line-range links are mostly cosmetic

`fspec show-coverage cross-frontend-integration-test-against-stub-provider`
reports 100% coverage, but multiple scenarios link to the same wide
range in `cross_frontend_parity.rs:283-408` (e.g. scenarios 4, 6, 7, 8,
9). That range is the body of `scenario_scripted_run_matches_golden`
plus its normalisation module — it is not the implementation of those
distinct scenarios. The links satisfy the audit but do not provide
useful traceability.

**Resolution:** Documenting; coverage links exist and are not strictly
wrong (the scripted_run body does exercise the normalisation pipeline),
just imprecise. Tightening them is low-value compared to the C1-C5
issues.

---

### W5. `scenario_send_input_hello_yields_canned_stream` is marked `#[ignore]` with a misleading reason

The `#[ignore]` reason reads: *"RPC-066: requires fspec binary built
with --features test-stub-provider"*. The real reason it cannot pass is
**C2 + C3** — even with the feature flag the agent loop is not wired
and the manager has no path to dispatch through the in-memory stub
registry.

**Resolution:** Documenting; not changing the `#[ignore]` line because
that would mask the real failure mode. The accurate label is "blocked
on sibling cards for agent-loop wiring + provider-manager stub dispatch".

---

### W6. The card's status is `done` but key deliverables are unmet

- Rule [10] golden fixture: missing
- Architecture note [B] ToolUse branch: deferred
- Architecture note [L] noop_tool: missing
- Architecture note [J] manager match-arm extension: half-done
- 3 of 4 integration tests fail when invoked with the right feature flag

A strict reading argues the card should be in `validating` (or even
`implementing`) pending the sibling cards. The pragmatic reading
honours architecture note [I]'s split-into-siblings allowance: the
**scaffolding** is delivered, but the **integration assertions** are
sibling-card territory.

**Resolution:** User instruction was "strictly to the requirements of
this card — no scope creep". I am **not** moving the card status. I am
flagging the discrepancy and recommending the user create sibling cards
explicitly tracking the unmet deliverables (recommended titles below).

---

## 🟢 Observations (Nice to Have)

### O1. The normalisation pipeline is a genuine behavioural test

`scenario_normalise_chunk_stream_substitutes_volatile_fields` constructs
real `StreamChunk` variants, runs `normalise::normalise_chunk_stream`,
and asserts on the JSONL output. This is the **only** non-#[ignore]'d
non-source-shape test in the file, and it is well-written. The
substitution logic handles `correlation_id`, `correlationId`,
`tool_call_id`, `toolCallId`, UUID pattern matching, and RFC-3339
pattern matching correctly.

### O2. Cargo.toml feature gating is clean

The `test-stub-provider` feature on `codelet-fspec` correctly enables
`dep:codelet-providers` (optional dep) plus
`codelet-providers/test-support`. The `#[cfg(feature = "test-stub-provider")]`
block in `common.rs::build_service` is well-isolated and compiles out
of release builds. The `no_napi_dependency.rs` regression test
infrastructure already enforces the boundary.

### O3. README.md regeneration procedure is clear

`codelet/fspec/tests/README.md` documents the `FSPEC_RPC_066_REGENERATE=1`
flow, the test inventory, and a "Future: TS-recorded reference fixture"
section. The TS-side path is deferred with a reasonable rationale.

---

## Recommended Sibling Cards (NOT created by this review)

The user instructed "no scope creep" so these are recommendations only.
The work below is explicitly authorised by architecture note [I] of
RPC-066 ("If the test surfaces blocking bugs, split them into sibling
cards rather than fixing inline.").

1. **RPC-NNN — Wire FspecSessionManagerHooks::spawn_agent_loop** — the
   no-op stub must be replaced with the same agent-loop dispatcher the
   NAPI binary uses, so the WS surface drives the real loop. Required
   for C2.
2. **RPC-NNN — Extend ProviderManager match arm for ProviderType::Custom
   to consult the in-memory stub registry** — call
   `codelet_providers::stub_provider::get_stub_provider(slug)` from the
   manager's existing `ProviderType::Custom` arm, falling back to the
   `RhaiCustomProvider` path. Required for C3.
3. **RPC-NNN — Implement StubProvider ToolUse path + register_noop_tool**
   — close out architecture notes [B] and [L]. Required for C4 and C5.
4. **RPC-NNN — Record cross_frontend_run.jsonl golden fixture** —
   blocked on the three cards above. Required for C1.
5. **RPC-NNN — Wire deny-network tracing assertion** — for W3.

---

## Fixes Applied by This Review

### F1. Remove false `register_noop_tool` deliverable from test docstring

`codelet/fspec/tests/cross_frontend_parity.rs:14-25`: updated module
docstring to accurately list only the surfaces that exist after this
card, and add a "Deferred to sibling cards" section enumerating C4/C5.

### F2. Fix incorrect RPC-067/068 sibling reference in stub_provider.rs

`codelet/providers/src/stub_provider.rs:80-101`: replaced the comment
that claims RPC-067/068 will add the ToolUse return + noop_tool
registration. Those cards address different deliverables. The corrected
comment defers to "a future sibling card under RPC-030" without a
specific id.

### F3. Document the tautological nature of source-shape scenarios

Updated the per-scenario doc comments to explicitly label the W1a–W1d
tests as **source-shape regression** (file-content assertions) rather
than behavioural assertions, so a future reader is not misled.

---

## Files Reviewed

- `spec/features/cross-frontend-integration-test-against-stub-provider.feature`
- `codelet/fspec/tests/cross_frontend_parity.rs`
- `codelet/fspec/tests/README.md`
- `codelet/fspec/tests/common/mod.rs`
- `codelet/fspec/src/common.rs`
- `codelet/fspec/src/session_hooks.rs`
- `codelet/fspec/Cargo.toml`
- `codelet/providers/src/stub_provider.rs`
- `codelet/providers/src/manager.rs` (selected sections)
- `codelet/providers/src/models/cache.rs`
- `codelet/providers/src/models/registry.rs`
- `codelet/sessions/src/handle_impl.rs` (create_session path)
- `codelet/sessions/src/session_manager.rs` (create_session_with_id path)
- `codelet/rpc/src/lib.rs` (RPC create_session handler)
- `codelet/tools/src/mcp.rs` (init_mcp_session)
- `spec/attachments/RPC-066/cross-frontend-integration-test.md`
- `spec/attachments/RPC-066/ast-research-stub-provider-and-daemon-surface.md`
- `spec/work-units.json` entry for RPC-066, RPC-067, RPC-068
