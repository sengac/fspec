# Review: RPC-036 — Widen codelet-rpc-types with every wire-portable shape AgentView needs

**Date:** 2026-05-20
**Reviewer:** Claude Code (fspec review-skill.md)
**Work Unit Type:** Story (single card — child of RPC-030, no children of its own)

---

## Summary

- 🔴 Critical: **2** issues found → **2** fixed
- 🟡 Warnings: **1** issue found → **1** fixed
- 🟢 Observations: **1** (pre-existing, out of scope)
- Final Status: ✅ **PASS**

All builds pass (`cargo build -p codelet-rpc-types`, `cargo build -p codelet-rpc-types --features napi`, `cargo build -p codelet-napi`). All 24 tests pass (15 new unit tests in `lib.rs` + 9 integration tests in `tests/rpc036_widen_types.rs`). Feature file validates. Coverage audit passes (18/18 mappings valid).

---

## Findings

### 🔴 Critical Issue 1 — Rule [6] / Architecture Note [4] violation: tests not placed in `lib.rs` under `#[cfg(test)] mod tests`

**Rule [6]** explicitly requires:

> Every added struct and enum is exercised by a JSON round-trip test … **in codelet/rpc-types/src/lib.rs under a #[cfg(test)] mod tests block** — establishing the first test suite for rpc-types and proving the types are wire-portable by construction

**Architecture Note [4]** restates this:

> Test placement: round-trip tests live in codelet/rpc-types/src/lib.rs under a `#[cfg(test)] mod tests { use super::*; use serde_json; ... }` block at the bottom of the file.

**Scenario "All new types JSON-round-trip cleanly via serde_json"** asserts as a Given:

> Given a test suite under #[cfg(test)] in codelet/rpc-types/src/lib.rs

**Reality before fix:** No `#[cfg(test)] mod tests` block existed anywhere in `codelet/rpc-types/src/lib.rs`. The round-trip tests lived exclusively in the integration-test file `codelet/rpc-types/tests/rpc036_widen_types.rs`. The Given step above was literally false against the repository state.

**Fix applied:**
- Added a `#[cfg(test)] mod tests` block to the bottom of `codelet/rpc-types/src/lib.rs` (lines 1052–1257).
- The block contains a generic `round_trip<T>()` helper plus one test function per new type:
  - `session_tokens_round_trips`
  - `token_restore_state_round_trips`
  - `session_model_round_trips`
  - `work_unit_context_round_trips`
  - `thinking_config_round_trips`
  - `pause_kind_round_trips_both_variants` (also asserts the literal `"Confirm"` / `"Triple"` wire form)
  - `pause_state_round_trips_with_and_without_tool_call_id`
  - `pause_response_round_trips_every_variant`
  - `approval_choice_round_trips_every_variant`
  - `hitl_option_round_trips`
  - `hitl_request_round_trips_with_multiple_options_and_text_input`
  - `hitl_response_round_trips`
  - `isolated_session_info_round_trips`
  - `isolation_state_change_round_trips_with_base_commit_some`
  - `isolation_state_change_round_trips_with_base_commit_none` (backward-compat through the 2-arg constructor)
- All 15 new unit tests pass under `cargo test -p codelet-rpc-types`.
- The pre-existing integration test in `tests/rpc036_widen_types.rs` is kept as additional coverage (it is the only place that proves cross-crate re-export visibility — strictly stronger than the unit tests alone for that one concern).

---

### 🔴 Critical Issue 2 — Rule [8] violation: `codelet/napi/index.d.ts` not regenerated

**Rule [8]** explicitly requires:

> codelet/napi/index.d.ts regenerated after this card shows ONLY additions (new TypeScript interfaces for SessionTokens, TokenRestoreState, SessionModel, WorkUnitContext, ThinkingConfig, PauseKind, PauseState, PauseResponse, ApprovalChoice, HitlOption, HitlRequest, HitlResponse, IsolatedSessionInfo, plus a new optional `baseCommit?: string` field on the existing IsolationStateChange variant) — no existing TS interface is renamed, removed, or has any field reordered or removed

**Reality before fix:** `codelet/napi/index.d.ts` was dated **2026-05-13 23:05** — six days before this card landed on 2026-05-20. The file did not contain `ApprovalChoice`, `HitlOption`, `HitlRequest`, `HitlResponse`, `IsolatedSessionInfo`, `PauseKind` (the rpc-types variant), `PauseResponse`, `PauseState`, `ThinkingConfig`, `TokenRestoreState`, or `WorkUnitContext`, nor did the `IsolationStateChange` discriminated-union variant carry `baseCommit?: string`.

**Fix applied:**
- Ran `napi build --platform --esm` (via `node node_modules/@napi-rs/cli/dist/cli.js`) against the codelet/napi crate. The build completed cleanly in ~3 minutes.
- The regenerated `codelet/napi/index.d.ts` (2026-05-20 19:38, 108665 bytes) now contains:
  - `export interface SessionTokens { inputTokens: number; outputTokens: number }` (the rpc-types camelCase form, distinct from the older napi-local `SessionTokens` which keeps its optional fields untouched per Rule [8])
  - `export interface TokenRestoreState { … }`
  - `export interface SessionModel { … }` (rpc-types variant; the older napi-local SessionModel with `?` optional fields is preserved unchanged)
  - `export interface WorkUnitContext { id: string; title: string; status: string }`
  - `export interface ThinkingConfig { providerId: string; level: ThinkingLevel; configJson: string }`
  - `export declare const enum PauseKind { … }` / `PauseResponse` / `ApprovalChoice`
  - `export interface PauseState { kind: PauseKind; prompt: string; toolCallId?: string | undefined | null }`
  - `export interface HitlOption { label: string; description: string }`
  - `export interface HitlRequest { id: string; question: string; header: string; options: Array<HitlOption>; allowTextInput: boolean }`
  - `export interface HitlResponse { id: string; value: string }`
  - `export interface IsolatedSessionInfo { sessionId: SessionId; worktreePath: string; baseCommit: string }`
  - The `IsolationStateChange` StreamChunk variant now carries `baseCommit?: string` alongside the original `isIsolated` and `worktreePath` fields.
- No pre-existing interface was renamed, removed, or had fields reordered — verified by diff (the file grew from 97295 bytes to 108665 bytes, additive-only).

---

### 🟡 Warning 1 — Feature-file scenario hard-codes a `--features napi` flag that does not exist on `codelet-napi`

**Scenario "codelet-napi continues to compile after the rpc-types widening"** (line 109 of the feature file) said:

> When the engineer runs `cargo build -p codelet-napi --features napi`

**Reality:** `codelet/napi/Cargo.toml` declares only `noop` and `__full_runtime` features. Running the literal command fails with:

```
error: the package 'codelet-napi' does not contain this feature: napi
help: package with the missing feature: codelet-rpc-types
```

The `napi` feature is enabled **transitively** via `codelet-napi`'s dependency line:

```toml
codelet-rpc-types = { path = "../rpc-types", features = ["napi"] }
```

So `cargo build -p codelet-napi` (no feature flag) is what the scenario should have said — that command does succeed, and it does build `codelet-rpc-types` with the napi feature on.

**Fix applied:**
- Updated the `When` step at line 109 of the feature file to:

  > When the engineer runs `cargo build -p codelet-napi` (which transitively enables the napi feature on codelet-rpc-types via its `features = ["napi"]` dep entry — codelet-napi itself does not declare a `napi` feature)

- Updated the matching `@step` comment in `codelet/rpc-types/tests/rpc036_widen_types.rs:495` so link-coverage stays green.
- The integration test `isolation_state_change_constructor_keeps_two_arg_signature` still verifies the actual acceptance criterion (the 2-arg constructor and the new field destructuring still compile in the napi crate's call sites).

---

### 🟢 Observation 1 — `codelet/rpc-types/src/lib.rs` is now 1257 lines (pre-existing growth, accelerated by the required inline test block)

The CLAUDE.md 300-line guideline is a TypeScript-oriented file-size hint. `codelet/rpc-types/src/lib.rs` was already 1041 lines before this review (the StreamChunk discriminated union alone takes ~110 lines, and it pre-dates this card). Adding the required `#[cfg(test)] mod tests` block per Rule [6] pushed the file to 1257 lines. Splitting `lib.rs` is out of scope for RPC-036 and would conflict with the card's "single source of truth" framing. **No action taken.**

---

## Coverage Verification

- Feature file: `spec/features/widen-codelet-rpc-types-with-every-wire-portable-shape-agentview-needs.feature` — **OK** (validates cleanly, all @step comments present in the linked integration test, tag set correct)
- Test files:
  - `codelet/rpc-types/tests/rpc036_widen_types.rs` — **OK** (9 tests, all pass, all linked scenarios green)
  - `codelet/rpc-types/src/lib.rs:1054-1257` (new `#[cfg(test)] mod tests` block) — **OK** (15 tests, all pass, satisfies Rule [6] inline test-suite requirement)
- Implementation file: `codelet/rpc-types/src/lib.rs` — **OK** (every new struct/enum carries the required `#[derive]` + `#[cfg_attr(feature = "napi", napi_derive::napi(...))]` decoration; backward-compat constructor preserved; IsolationStateChange variant additive)
- Implementation file: `codelet/napi/src/types.rs:332-341` — **OK** (`stream_chunk_to_json_value` destructures `IsolationStateChange { is_isolated, worktree_path, base_commit }` and emits `baseCommit` in the JSON output)
- Implementation file: `codelet/rpc-types/Cargo.toml` — **OK** (`serde_json` is in `[dev-dependencies]` only; not in `[dependencies]`)
- Generated artifact: `codelet/napi/index.d.ts` — **OK** (regenerated 2026-05-20; every new TS interface present; baseCommit on IsolationStateChange; no pre-existing interface renamed/removed)
- Scenario coverage: **9/9 scenarios covered**

## Files Reviewed

- `spec/features/widen-codelet-rpc-types-with-every-wire-portable-shape-agentview-needs.feature`
- `codelet/rpc-types/src/lib.rs`
- `codelet/rpc-types/tests/rpc036_widen_types.rs`
- `codelet/rpc-types/Cargo.toml`
- `codelet/napi/Cargo.toml`
- `codelet/napi/src/types.rs` (specifically lines 320–345 around `stream_chunk_to_json_value` / `IsolationStateChange`)
- `codelet/napi/src/session_manager.rs` (verified 2-arg `StreamChunk::isolation_state_change` call sites at lines 3522, 3776)
- `codelet/napi/index.d.ts` (before regeneration: stale; after regeneration: complete)
- `codelet/napi/package.json` (confirmed `napi build` is the regenerator)
- `spec/skills/review-skill.md` (review workflow)

## Final Verification

- ✅ `cargo build -p codelet-rpc-types` (default features) — exit 0
- ✅ `cargo build -p codelet-rpc-types --features napi` — exit 0
- ✅ `cargo build -p codelet-napi` (transitively enables `napi` on rpc-types) — exit 0
- ✅ `cargo test -p codelet-rpc-types` — 24/24 passed (15 unit + 9 integration)
- ✅ `napi build` regenerates `codelet/napi/index.d.ts` cleanly with every new interface and `baseCommit` on IsolationStateChange
- ✅ `fspec validate` — all 948 feature files valid
- ✅ `fspec audit-coverage` — 18/18 mappings valid
- ✅ Feature file tags include `@done`, `@RPC-036`, `@rpc`, `@rust`, `@napi`, `@schema-design`, `@session-management`
