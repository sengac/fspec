# RPC-039 Review Findings

**Date:** 2026-05-21
**Reviewer:** Claude Code (fspec review skill)
**Work Unit:** RPC-039 — Move BackgroundSession from codelet-napi into codelet-sessions, replace NAPI references
**Scope:** Single work unit (no children — RPC-039 is a leaf child of RPC-030)

---

## Summary

- 🔴 Critical: 3 issues, all fixed
- 🟡 Warnings: 2 issues, both fixed
- 🟢 Observations: 2 (no action required, scope-deferred)

---

## Review: RPC-039 — Move BackgroundSession from codelet-napi into codelet-sessions

### Status: ✅ PASS (after fixes)

### 🔴 Critical Issues (Found and Fixed)

1. **Coverage test line ranges off by ~19–66 lines** — every scenario's recorded
   `testLines` pointed to an earlier line range than the actual `#[test] fn …`
   span in `codelet/sessions/tests/background_session_shape.rs`. Example:
   scenario 1 was recorded at `67–113` but the actual test body lives at
   `86–128`. The drift accumulates so by scenario 10 the recorded `653–769`
   was off by ~66 lines (actual `719–835`).
   - **Fix:** unlinked every scenario via `fspec unlink-coverage --all` and
     relinked with the real line ranges discovered by AST walking the test
     file (`86-128`, `134-189`, `195-228`, `235-310`, `316-375`, `382-461`,
     `467-551`, `558-633`, `640-713`, `719-835`).

2. **NAPI re-export coverage line range pointed at the wrong code** — five
   scenarios referenced `codelet/napi/src/session_manager.rs:92-103`. Lines
   92-103 are the `impl GlobalChunkCallback { … }` block, NOT the
   `pub use codelet_sessions::background_session::{ … }` re-export the
   scenarios are actually about. The real re-export block is at lines 118-121
   (with its doc comment starting at line 113).
   - **Fix:** unlinked and relinked each affected scenario with
     `codelet/napi/src/session_manager.rs:113-121`.

3. **`session_send_input` NAPI shim coverage line range pointed at random
   code** — scenario 5 ("send_input is rewritten to a non-NAPI Result type")
   linked `codelet/napi/src/session_manager.rs:5525-5538`, but the actual
   `pub fn session_send_input` shim that maps the new `Result<(), String>`
   error back to `napi::Error::from_reason` lives at lines 5556-5562 (with
   the `#[napi]` attribute on 5556).
   - **Fix:** relinked with `codelet/napi/src/session_manager.rs:5556-5562`.

### 🟡 Warnings (Found and Fixed)

1. **`chunks_tx` field type deviated from rule [6] of the example map.**
   Rule [6] specifies the field type as
   `Option<tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>>`. The
   implementation had `RwLock<Option<broadcast::Sender<…>>>`. The extra
   `RwLock` was added in anticipation of a `set_chunks_tx` method that does
   not exist in this card (and is explicitly RPC-041's responsibility).
   - **Fix:** simplified the field to plain
     `Option<broadcast::Sender<(codelet_rpc_types::SessionId, StreamChunk)>>`,
     simplified the `handle_output` read site to `if let Some(tx) = &self.chunks_tx`,
     and updated the constructor initializer from `RwLock::new(None)` → `None`.
     All 10 tests in `background_session_shape.rs` still pass; `cargo build -p
     codelet-napi` and `cargo build -p codelet-sessions` both still succeed.

2. **Doc comment on `chunks_tx` falsely promised a `set_chunks_tx` method.**
   The pre-fix doc string asserted "`BackgroundSession::set_chunks_tx` is
   provided so RPC-041 can supply the sender …" — but no such method existed
   anywhere in the file. This was misleading future-context for any reader.
   - **Fix:** rewrote the doc string to say "RPC-041 will decide how to
     populate this field — either by widening `new()` or by introducing a
     dedicated setter." This matches the actual RPC-039 deliverable
     (defaulted-to-None field) without overpromising.

### 🟢 Observations (No Action — Out of Scope)

1. `codelet/sessions/src/background_session.rs` is **1194 lines**, far
   exceeding the 300-line guideline in `CLAUDE.md`. This file is a verbatim
   move (rule [9] forbids behavioural changes) and the architecture notes
   defer refactoring explicitly to RPC-040 (move SessionManager) and RPC-042
   (implement SessionManagerHandle). Out of scope for this card.

2. The file uses several relaxed clippy `#![allow(...)]` attributes
   (`expect_used`, `unwrap_used`, `redundant_clone`, etc.). These are
   carry-overs from `codelet-napi` (which did not enforce these lints) and
   are explicitly annotated as "cleanup is tracked by RPC-040 / RPC-042". Out
   of scope.

---

## Coverage Verification (Post-Fix)

| Layer | Path | Status |
|-------|------|--------|
| Feature file | `spec/features/move-background-session-into-codelet-sessions.feature` | ✅ OK — 10 scenarios, valid Gherkin, `@RPC-039` tag present, architecture doc string present |
| Test file | `codelet/sessions/tests/background_session_shape.rs` | ✅ OK — 10 `#[test]` functions, `// @step` comments match every Gherkin step verbatim, all 10 tests pass after fix |
| Impl files | `codelet/sessions/src/background_session.rs`, `codelet/napi/src/session_manager.rs`, `codelet/sessions/Cargo.toml` | ✅ OK — line ranges now point at the real code each scenario asserts |
| Scenario coverage | 10/10 (100%) | ✅ OK |

---

## Files Reviewed

- `spec/features/move-background-session-into-codelet-sessions.feature`
- `spec/features/move-background-session-into-codelet-sessions.feature.coverage`
- `codelet/sessions/tests/background_session_shape.rs`
- `codelet/sessions/tests/smoke.rs`
- `codelet/sessions/src/background_session.rs`
- `codelet/sessions/Cargo.toml`
- `codelet/napi/src/session_manager.rs` (line 78 `GLOBAL_CHUNK_CALLBACK`, line 118-121 re-export, line 5557-5562 `session_send_input`)
- `spec/attachments/RPC-039/move-background-session.md`
- `spec/attachments/RPC-039/ast-research-background-session-move.md`

---

## Rule-by-Rule Compliance (Post-Fix)

| Rule | Status | Evidence |
|------|--------|----------|
| [0] BackgroundSession struct + impl lives verbatim in `codelet/sessions/src/background_session.rs` | ✅ | lines 270-1194 |
| [1] No `napi::` references in the moved file | ✅ | only in doc comments (stripped by tests); zero in executable code |
| [2] No `crate::persistence::` imports in moved file | ✅ | verified |
| [3] `FspecResult` resolves to `codelet_rpc_types::FspecResult` | ✅ | line 82: `use codelet_rpc_types::{FspecResult, …};` |
| [4] `napi::Error::from_reason` rewritten to `format!` | ✅ | line 1097: `format!("Failed to send input: {}", e)` |
| [5] Supporting types moved + napi `pub use` re-exports them | ✅ | `codelet/napi/src/session_manager.rs:118-121` |
| [6] `chunks_tx: Option<broadcast::Sender<…>>` field added (defaulted None) | ✅ (after fix) | line 338 (was `RwLock<Option<…>>`; now plain `Option<…>`) |
| [7] `GLOBAL_CHUNK_CALLBACK` survives in napi shell | ✅ | `codelet/napi/src/session_manager.rs:78` |
| [8] All builds succeed | ✅ | `cargo build -p codelet-sessions` and `cargo build -p codelet-napi` both finish clean |
| [9] No behavioural changes (only `send_input` return-type widening) | ✅ | inspected |

---

## Fix Results

### RPC-039 — Move BackgroundSession from codelet-napi into codelet-sessions

- 🔴 Coverage test line ranges drift → ✅ Fixed: unlinked and relinked all 10 scenarios.
- 🔴 NAPI re-export coverage pointed at wrong block (92-103 instead of 113-121) → ✅ Fixed.
- 🔴 `session_send_input` coverage pointed at wrong span (5525-5538 instead of 5556-5562) → ✅ Fixed.
- 🟡 `chunks_tx` field had unnecessary `RwLock` wrapper (deviation from rule [6]) → ✅ Fixed: simplified to plain `Option<…>`.
- 🟡 Misleading doc comment promised non-existent `set_chunks_tx` method → ✅ Fixed: rewrote doc to defer the decision to RPC-041 without overpromising.

## Final Verification

- ✅ `cargo build -p codelet-sessions` succeeds
- ✅ `cargo build -p codelet-napi` succeeds
- ✅ `cargo test -p codelet-sessions --test background_session_shape` — all 10 tests pass (46 s)
- ✅ `fspec validate` — 951 feature files valid (no Gherkin errors)
- ✅ Coverage is now correctly linked at the real line ranges
