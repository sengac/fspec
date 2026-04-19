# Review: PROV-083 — OpenAI Chat Completions rejects base64 user images with detail=None

**Date:** 2026-04-19
**Reviewer:** Claude Code (fspec review skill, single-work-unit mode)
**Scope:** PROV-083 only (no children)

## Status: ✅ PASS

## 🔴 Critical Issues (Must Fix)
None.

## 🟡 Warnings (Should Fix)
None.

## 🟢 Observations (Nice to Have)

1. **File size pre-existing condition** — `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` is 1986 lines, well over the project's 300-line guideline. This is a vendored/patched upstream file (rig-core fork) and PROV-083 only added 24 lines of test code inside an isolated `#[cfg(test)] mod prov_083_tests` block plus a 2-line fix at 445. No net refactoring pressure created by this work unit; flagging only for epic-level tracking.
2. **Test asserts beyond feature text** — `prov_083_base64_png_with_detail_none_defaults_to_auto` also asserts the serialized JSON shape (`wire_json["type"] == "image_url"`). This exceeds what the Gherkin requires but is additive and useful (catches silent tag renames). Not a defect.
3. **Helper `convert()` is a thin wrapper** over `UserContent::try_from` — fine for readability, mild DRY concern if more suites are added; acceptable for the scope.

## Coverage Verification

- **Feature file:** `spec/features/openai-provider-base64-image-inputs.feature` — OK
  - `@PROV-083` tag present (line 2), `@done` tag present (line 1)
  - Architecture doc string present (lines 5-8), accurate, references the exact fix site
  - Example Mapping context block present (lines 10-27) with all 4 rules and 5 examples
  - Background user-story format correct (lines 29-32)
  - All 5 scenarios have correct Given/When/Then ordering — no And-after-Then preconditions
  - No placeholder text (`[role]`, `[action]`, `[benefit]`) detected
- **Test file:** `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` lines 1803-1986 — OK
  - Module header references feature file (line 1805): "Feature: spec/features/openai-provider-base64-image-inputs.feature"
  - 5 tests, one per scenario, correctly mapped
  - All 24 @step comments match feature file step text exactly (verified via `diff` against scenario steps 35-71; no deviations)
  - Negative-path test (media_type=None) uses `expect_err` and checks both the MIME message AND that the error does NOT reference "image detail" — this is a good regression guard for the original bug signature
- **Impl file:** `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` lines 428-451 — OK
  - URL branch (428-433) unchanged — uses `detail.unwrap_or_default()` as documented
  - Base64 branch (434-451) — the `let detail = detail.ok_or(ConversionError("OpenAI image URI must have image detail"))?` has been replaced with `let detail = detail.unwrap_or_default();` at line 445. Exactly the one-line swap specified in architecture note [0]
  - No other edits to this function (verified via `git diff HEAD` — the remaining hunks in the file are PROV-081 reasoning-field work in separate regions, not PROV-083)
- **Scenario coverage:** 5/5 — 100% per `fspec show-coverage openai-provider-base64-image-inputs`

## Check Results

### A. Feature File Compliance — PASS
- Step ordering correct on every scenario
- Architecture doc string present
- `@PROV-083` tag present
- No prefill placeholders

### B. Example Map Alignment — PASS
- 4 rules → addressed by scenarios 1/3 (rule 0,1), 3 (rule 1), 1/2 (rule 2), 5 (rule 3)
- 5 examples → 5 scenarios, 1:1 mapping
- 0 questions (red cards) remaining
- 2 architecture notes match implementation: note [0] (line-level fix) verified at line 445; note [1] (`mod prov_083_tests` location) verified at line 1804

### C. Test Coverage — PASS
- 5/5 scenarios have tests
- 24/24 @step comments match Gherkin step text exactly (verified by diff against feature file)
- Tests verify real conversion behaviour (data shape, detail value, serialized JSON, error variant + message substring) — no trivial assertions

### D. Implementation Quality — PASS
- **Single-line fix confirmed**: The delta touching the fix site is exactly 3 lines removed / 1 line added at mod.rs:430-446 (the `ok_or(...)?` → `unwrap_or_default()` swap). No incidental edits to adjacent code.
- No `unwrap()`, `todo!()`, or `unimplemented!()` introduced
- `ImageDetail::default()` producing `Auto` matches OpenAI's documented server default — behavioural parity verified against rule [0]
- No `any`-type equivalents (n/a — Rust)
- No dead code introduced

### E. Build & Test — PASS
- `cargo build --lib` — clean (`Finished dev profile`)
- `cargo test --lib` — **215 passed, 0 failed, 3 ignored** (matches the 215/0/3 target stated in the task)
- All 5 PROV-083 tests pass, including:
  - `prov_083_base64_png_with_detail_none_defaults_to_auto`
  - `prov_083_base64_jpeg_with_detail_none_defaults_to_auto`
  - `prov_083_base64_png_with_explicit_high_preserves_value`
  - `prov_083_url_image_path_unchanged` ✅ (confirms no URL-path regression per check F)
  - `prov_083_base64_with_media_type_none_still_errors_on_mime`

### F. Cross-Cutting — PASS
- URL-path regression test (`prov_083_url_image_path_unchanged`) passes, confirming rule [3] / example [3] invariant
- No security/performance concerns introduced (same encoding, same error surface, no new network paths)
- Implementation matches architecture notes verbatim

## Files Reviewed

- `spec/features/openai-provider-base64-image-inputs.feature` (full)
- `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` lines 400-460 (fix site)
- `codelet/patches/rig-core/src/providers/openai/completion/mod.rs` lines 1800-1986 (test module)
- `git diff HEAD` of the full file to verify scope isolation

## Source Modification Confirmation

**I did NOT modify any source files during this review.** Only wrote this findings document to `spec/attachments/PROV-083/review-findings.md`.
