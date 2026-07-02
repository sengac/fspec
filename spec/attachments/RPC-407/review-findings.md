# Review Findings: RPC-407 — Rust binary never initializes project blocklist root

**Date:** 2026-07-02
**Reviewer:** Claude Code (fspec review skill, parallel reviewer 367d948e)
**Status:** WARN (0 critical, 3 warnings, 7 observations)

## 🔴 Critical Issues
None. Fix verified correct: `build_service` (common.rs:90-101) is the shared chokepoint for daemon (daemon.rs:38) and combined (combined.rs:35); all 3 RPC-407 tests pass; 35/35 bin tests green; pre-existing `cargo_shape.rs`/`cli_check.rs` failures verified unrelated (caused by earlier commits 9ac298d4/dc623808: `init_selector.rs` + `attachment-viewer` layout drift, formatting fixture).

## 🟡 Warnings (Should Fix)
1. **Tests do not redirect `HOME`** despite the design attachment (§3) explicitly calling for it. `blocklist_init_tests.rs:72,106,117` call `check_bash_command`, which merges the real `~/.fspec/blocklist.json` — a system rule matching the sentinel (or a broad `prompt` rule) could fail the negative control or hang in `pause_for_user` on someone else's machine/CI. Fix: point `HOME` at a temp dir for the duration of each test (serial tests, so env mutation is safe) and restore it.
2. **Global-state restoration is not panic-safe** — `restore_global_blocklist_state()` is called manually (lines 95, 118); an assertion failure mid-test leaves `BLOCKLIST_PROJECT_ROOT` pointing at a deleted tempdir for subsequent `#[serial]` tests, contradicting the file's own header promise. Fix: RAII `Drop` guard that restores unconditionally.
3. **Work unit is `done` but all RPC-407 files are uncommitted** (untracked feature file + test file; modified common.rs/main.rs/Cargo.toml/cargo_shape.rs). NOTE (supervisor): the repo carries substantial uncommitted work from several other done cards (RPC-400..406, 408); committing is deferred to the repo owner — recorded here rather than fixed by the review worker.

## 🟢 Observations (Nice to Have)
1. Feature title is bug-phrased ("Rust binary never initializes…") — capability phrasing ("Project blocklist initialization at service startup") reads better as living documentation.
2. Coverage impl lines 90-101 are mostly comments; only :101 is executable. Accurate but inflated.
3. Pre-existing test failures belong to a separate cleanup card (init_selector.rs/attachment-viewer lock-list drift; formatting fixture).
4. Worktree-isolated sessions share the single process-global root — documented non-goal, matches napi.
5. No DRY issue: two thin call sites into shared `init_blocklist`.
6. common.rs at 849 lines is a documented pre-existing exception (900-line cap in cargo_shape.rs).
7. Scenario 3's string-surgery body extraction is brittle but consistent with crate convention.

## Coverage Verification
- Feature file: spec/features/project-blocklist-initialization.feature — OK
- Test file: codelet/fspec/src/blocklist_init_tests.rs — OK (@step comments exact, real assertions, serial)
- Impl: codelet/fspec/src/common.rs:90-101 — OK
- Scenario coverage: 3/3

## Fix Results (2026-07-02, remediation worker 2fc6fd5f)

- **W1 (HOME not redirected) — FIXED.** `GlobalBlocklistGuard` in `codelet/fspec/src/blocklist_init_tests.rs` now redirects `HOME` to a fresh empty tempdir on construction (tests are `#[serial]`, env mutation safe) so the real `~/.fspec/blocklist.json` can never merge into `check_bash_command`; the prior `HOME` is restored on drop.
- **W2 (restoration not panic-safe) — FIXED.** Manual `restore_global_blocklist_state()` calls replaced by the RAII `Drop` impl on `GlobalBlocklistGuard` (restores `BLOCKLIST_PROJECT_ROOT` via `init_blocklist(None)`, clears session allowances, restores/removes `HOME`) — runs even when an assertion unwinds. File header comment updated to describe the guard.
- **Obs1 (bug-phrased feature title) — FIXED.** Feature retitled to "Project blocklist initialization at service startup" (filename unchanged; coverage keyed by filename verified intact via show-coverage, 3/3).
- **W3 (uncommitted work) — DEFERRED by supervisor.** Nothing committed; repo owner to commit.
- **Verification:** `cargo test -p codelet-fspec` — 3/3 RPC-407 tests pass, no NEW failures (pre-existing `cargo_shape`/`cli_check` failures unchanged, tracked separately); `cargo clippy -p codelet-fspec` clean; `cargo fmt` clean; coverage 100%. Card cycled done → implementing → validating → done.
