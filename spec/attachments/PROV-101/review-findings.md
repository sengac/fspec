# Review Findings — PROV-101 (Remove all provider/model/profile selection fallbacks)

**Reviewer:** spawned ACDD review worker (session 741b5bb9), supervised.
**Result:** PASS (with WARN). No 🔴 critical. Core mission verified: every silent selection
fallback (#1–#8) removed/de-privileged; 100% coverage; clippy `-D warnings` clean; all test-only
seeds verified `#[cfg(test)]`-gated and unreachable; `deferred_placeholder_provider` provably never
a silent default; `fallback_models.json` deleted.

## 🔴 Critical
None.

## 🟡 Warnings (TO FIX per user mandate)
1. **Empty-`SessionId` decline silently swallowed by callers.** `create_session` returns
   `SessionId::new(String::new())` when no default model, but no consumer guards it (rpc/src/lib.rs:793
   wraps in Ok; TUI dispatch.rs:58/93/97, dispatch_create_session_dialog.rs:95 treat any Ok as
   success and append an empty-id SessionContext). Hidden failure mode. → Convert to a typed error
   (symmetry with create_isolated_session) and/or have callers reject empty ids.
2. **Dead RPC-022 `ModelSelectorDialog` still carries the old index-0 fallback.**
   `model_selector_dialog.rs:49,69` and `model_selector_dialog_rows.rs:260,274` still use
   `.position(|r| r.selectable).unwrap_or(0)`. Constructed only via `Action::OpenModelDialog` which is
   never dispatched at runtime (replaced by `OpenModelSelectorView`). Dead but compiled/tested. The
   user mandate is REMOVE EVERY FALLBACK → delete the retired dialog (and its tests) so the fallback
   cannot resurface.
3. **300-line rule breached in touched megafiles:** manager.rs (2853), model_selector/mod.rs (2069),
   handle_impl.rs (1766), rows.rs (833). Pre-existing; PROV-101 added to them. → Assess extraction.

## 🟢 Observations
- `resolve_unambiguous_provider` clean: zero→auth Err, one→Ok, many→config Err mentioning "explicit".
- `index_of_model` guards on `r.selectable` (header rows can't false-match).
- All `// @step` comments byte-exact to Gherkin. Tests genuinely offline (temp dir + seeded fixture).

---

## Resolution of WARN findings (PROV-101 re-work, ACDD)

### FIX 1 — Empty-`SessionId` decline no longer swallowed ✅
- **Decision: caller-guard (the supervisor-authorized fallback), NOT a typed-error
  signature change.** Converting `SessionManagerHandle::create_session` /
  the RPC wire return type to `Result<SessionId, String>` has an unacceptable
  blast radius with no git safety net: the `SessionId` return is consumed at
  ~40+ call sites across `rpc-server`, `rpc-embedded`, `fspec`, `fspec-tui` and
  `napi` tests, plus the `FspecBackend` trait + 3 transport impls + the core
  trait + 2 trait impls. The handle still returns an empty `SessionId` on
  decline (rule [0] unchanged); the TUI now detects it explicitly.
- **New shared helper** `codelet/fspec-tui/src/app/session_creation.rs`:
  `post_create_session_action(SessionId) -> Action` maps a real id to
  `Action::SessionCreated` and an empty id to the new
  `Action::SessionCreationDeclined`; `route_bootstrap_create_session(...)`
  routes the bootstrap/lazy paths (never seeds an empty active session).
- **All three callers fixed:** `dispatch.rs` (Reconnected + EnterWorkUnit) and
  `dispatch_create_session_dialog.rs` (CreateSessionSubmitted, non-isolated).
- **Explicit surface:** `App::handle_session_creation_declined` pushes a
  Priority::Critical `ErrorDialog` ("Session creation declined: no default model
  is set…"). No caller appends an empty-id `SessionContext`.
- **Spec + tests (behavior change → ACDD):** new capability feature
  `spec/features/session-creation-decline-surfaced.feature` (3 scenarios) + new
  rule[6]/example[7] on the card; new offline test
  `codelet/fspec-tui/tests/prov101_session_creation_decline.rs` (3 tests incl.
  an end-to-end App-level proof the decline shows the dialog and appends no
  session). 100% coverage, byte-exact `// @step`.

### FIX 2 — Dead RPC-022 `ModelSelectorDialog` deleted ✅
- **Verified truly dead:** `Action::OpenModelDialog` has NO `send(...)` producer
  anywhere; `/model` dispatches `Action::OpenModelSelectorView` (the live
  full-screen `ViewMode::ModelSelector`). No live runtime path reaches the modal.
- **Deleted:** `components/model_selector_dialog.rs` (carried the index-0
  fallbacks at old lines 49,69); `page_step_selectable` in
  `model_selector_dialog_rows.rs` (carried the fallback at old line 260; only the
  dead dialog used it); the now-dead `build_rows` / `build_dialog_rows`;
  `Action::OpenModelDialog` + `handle_open_model_dialog` + the no-op
  `handle_list_providers_loaded` + their match arms; the `ModelSelectorDialog` /
  `MODEL_SELECTOR_DIALOG_ID` lib export + the `pub mod model_selector_dialog;`.
- **Note on flagged line 274 (`first_selectable`'s `.unwrap_or(0)`):** this helper
  is LIVE — it is the explicit Home/arrow nav anchor used by the full-screen
  mode-view (per the architecture note), NOT a silent selection default. It only
  yields 0 when there are zero selectable rows, and Enter is still gated by
  `has_active_selection`. Kept by design.
- **Test fallout cleaned** (the deletion cascaded into other cards' shape/parity
  tests — all shared files were trimmed, not blindly deleted, preserving their
  LIVE coverage; `ListProvidersLoaded`/`ModelSelected` Action variants remain —
  they are used by the live mode-view):
  - Deleted `tests/model_selector_dialog_rpc022.rs` + feature
    `rpc022-model-selector-dialog.feature` (+coverage) — 100% dead.
  - `rpc027_dialog_parity_ef.rs`: removed Section E (4 dialog scenarios); kept
    Section F (ConfirmDialog). 4 scenarios removed from
    `rpc027-model-confirm-dialogs.feature`.
  - `app_dispatch_rpc022.rs`: removed the 1 dialog-fold test; kept the live
    ModelSelected/ThinkingLevel/Role tests. 1 scenario removed from
    `rpc022-app-dispatch.feature`.
  - `rpc028_popup_scroll.rs`: removed the 1 ModelSelectorDialog PageDown test +
    helpers. 1 scenario removed from `rpc028-scroll-mouse-wrap-parity.feature`.
  - `source_shape_rpc022.rs` + `rpc022-source-shape.feature`: dropped the
    `model_selector_dialog.rs` exists / <300-lines / no-forbidden-import /
    `OpenModelDialog` assertions; coverage re-linked off the deleted file.
  - `behaviour_parity_rpc065.rs` + `slash_command_wiring_rpc022.rs`: the
    "modal must NOT appear" negative assertions now use the literal id string
    `"model-selector-dialog"` (the modal const is gone); `SlashCommandParse::
    OpenModelDialog` parse variant is LIVE (maps to the view) and is untouched.

### FIX 3 — 300-line rule ✅ (extract PROV-101-touched code; document residual)
- **Extracted** (clean, focused submodules):
  - `codelet/providers/src/provider_resolution.rs` (61 LoC) — the PROV-101
    `resolve_unambiguous_provider` lifted out of `manager.rs`.
  - `codelet/fspec-tui/src/app/session_creation.rs` (59 LoC) — the FIX 1 mapping
    helpers (new code, kept in its own module from the start).
  - The FIX 1 `handle_session_created` / `handle_session_creation_declined`
    handlers live in `dispatch_create_session_dialog.rs`, keeping
    `app/dispatch.rs` at 291 LoC (< 300).
  - `components/model_selector_dialog_rows.rs` shrank 282 → 102 LoC (FIX 2).
- **Residual PRE-EXISTING megafile overage (NOT rewritten — too risky, out of
  card scope, no git safety net). Recommend dedicated refactor cards:**
  | File | LoC | Note |
  |------|-----|------|
  | `codelet/providers/src/manager.rs` | 2805 (was 2853) | provider instantiation god-file; split per-provider getters + model cache wiring |
  | `codelet/fspec-tui/src/views/model_selector/mod.rs` | 2069 | mode-view + its inline test module; split tests via `#[path]` + extract render/nav |
  | `codelet/sessions/src/handle_impl.rs` | 1766 | `SessionManagerHandle` impl; split by capability group |
  | `codelet/fspec-tui/src/views/model_selector/rows.rs` | 833 | row projection/legend; split builder vs tests |

### Pre-existing failure surfaced (NOT caused by PROV-101, out of scope)
- `codelet/sessions/tests/session_manager_shape.rs` has 3 failures
  (`create_session_with_id`/`create_isolated_session_with_id` signature shape +
  one body-substring assertion). Root cause: those source-shape tests expect a
  SINGLE-LINE signature, but rustfmt wraps the (>100 char) signature in
  `session_manager.rs` across multiple lines. `cargo fmt -p codelet-sessions
  --check` is CLEAN, so the brittle test and rustfmt are mutually exclusive.
  `session_manager.rs` was NOT touched by PROV-101 (this session changed only
  codelet-providers + codelet-fspec-tui + spec/). Recommend a card to make those
  shape tests whitespace-insensitive.

### Gate evidence (re-run)
- `cargo test`: providers 329 lib + all integration `0 failed`; sessions PROV-101
  tests (`prov101_no_selection_fallbacks` 3, `rpc343` 7, `rpc348` 5, `rpc081` 5)
  `0 failed`; fspec-tui `--lib` 228 + affected integration (`prov101_session_
  creation_decline` 3, `rpc027` 4, `app_dispatch_rpc022` 8, `rpc028` 35,
  `source_shape_rpc022` 11, `behaviour_parity_rpc065` 28, `slash_command_wiring_
  rpc022` 10) `0 failed`; full fspec-tui suite `0 failed`.
- `cargo clippy -p codelet-providers -p codelet-sessions -p codelet-fspec-tui
  --all-targets -- -D warnings` → EXIT 0, 0 warnings.
- `cargo fmt --check` on all three crates → CLEAN.
- `cargo build` codelet-core + codelet-napi + codelet-fspec → Finished.
- `fspec validate` → all 1446 feature files valid.
- `fspec validate-tags` → all 4 PROV-101 features carry required component +
  feature-group tags (the repo-wide 479 violations are pre-existing template
  noise on unrelated features).
- `show-coverage` 100% on all 4 PROV-101 features; `audit-coverage` clean on all
  4 + the edited rpc022/rpc027/rpc028 features.
