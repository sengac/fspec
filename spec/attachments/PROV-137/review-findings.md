# Epic Review: PROV-137 / PROV-138 — Copy & Paste for /provider view input areas

**Date:** 2026-07-05
**Reviewer:** Claude Code (fspec review skill) via parallel subordinate reviewers
**Work Units Reviewed:** 2 (PROV-137 paste, PROV-138 copy)

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 1 actionable (PROV-137 Gherkin `Then`→`And`)
- 🟢 Observations: 5 (all non-blocking, no action required)

Both reviewers independently verified the **security requirement is airtight**: the plaintext API key never renders, never enters the Action bus, and never appears in the OSC 52 clipboard bytes. `mask_secret` is a true single source of truth shared by the render (profile form idx 1 + inline EditApiKey) and the copy path.

## Work Unit Results

### PROV-137: Paste support — PASS
- Security: masking single-sourced via `copy::mask_secret`; no new plaintext render path; newline/control chars stripped by the `is_printable_ascii` (32..=126) and `(' '..='~')` charset gates; end-to-end paste path wired from `app/events.rs` bracketed-paste arm → navigator → field sinks (not isolated to unit tests).
- Tests: 4/4 pass; @step comments verbatim; scenario-3 `\n` handling honest (feature carries literal `\ndef`, code uses a real newline, assertion proves the strip).
- Files all < 300 lines; clippy clean.
- 🟡 **Warning 1 (actionable):** feature file scenarios 2 & 3 use two consecutive `Then` steps where the second assertion should be `And` (valid Gherkin, but violates the review convention). Fix: convert the trailing `Then` to `And` and re-sync the test `@step` comments.
- 🟢 Observations: `push_paste_char` deliberate pub(crate) seam; `detail::handle_edit_paste` inlines a 2-line char loop (acceptable DRY); analysis doc slightly stale re: `insert_str` location.

### PROV-138: Copy support — PASS
- Security: `handle_copy` masks BEFORE building `Action::CopyToClipboard`; `mask_secret` is the ONLY `"•".repeat(...)` in src/ and is called by all three sites (copy, profile_form_render, detail render) with identical `chars().count()` semantics; app-level tests assert the exact OSC 52 bytes decode to the masked value AND do not contain the plaintext substring.
- Ctrl+C intercept ordered correctly BEFORE the blanket CONTROL/ALT consume; `is_copyable_mode` gates to input modes only; List/OAuth Ctrl+C emit no CopyToClipboard; OAuth 'c' copy-url path unaffected.
- Tests: 4/4 pass; full suite 237 suites / 0 failed (no PROV-137 regression; 300-LoC source-shape tests pass); clippy clean.
- 🔴/🟡: None blocking.
- 🟢 Observations: empty non-secret field emits `CopyToClipboard("")` (harmless, writes nothing); OAuth Ctrl separation is clean; the "empty field" example-map seed has no dedicated scenario (was a seed, not a final AC).

## Fix Results

### PROV-137
- 🟡 Warning 1 (Gherkin `Then`→`And`): ✅ FIXED. Feature scenarios 2 & 3 second assertions converted from `Then` to `And`; the two corresponding test `@step` comments re-synced verbatim (lines 154 & 202). Feature re-validates; tests 4/4 still pass; coverage remains 100%. Walked PROV-137 back through specifying→testing→implementing→validating→done (ACDD-proper).
- 🟢 Observations: no action (deliberate seams / acceptable DRY / stale-analysis note only).

### PROV-138
- 🔴/🟡: None — PASS with no actionable findings. No fix cycle required.
- 🟢 Observations: no action (empty-field emits `CopyToClipboard("")` harmlessly; OAuth Ctrl separation clean; "empty field" was a seed not an AC).

## Final Verification
- All targeted tests pass: ✅ (PROV-137 4/4, PROV-138 4/4)
- Full crate suite: ✅ (no regressions; 300-LoC source-shape tests pass)
- Clippy: ✅ clean
- Feature files valid: ✅
- Coverage complete: ✅ 100% both features
- Security (API key masked through render + paste + copy + action bus + OSC 52 bytes): ✅ airtight, single-sourced via `mask_secret`
