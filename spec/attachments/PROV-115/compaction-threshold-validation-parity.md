# PROV-115 — Profile compaction-threshold range-validation parity

**Date:** 2026-06-23
**Source:** DeepSearch of `src/tui/` + `src/utils/` (TypeScript reference) vs
`codelet/fspec-tui/src/views/` (Rust port), cross-checked by reading the actual
source. Every claim below carries a `file:line` reference — nothing fabricated.

---

## 1. The verified parity gap

### TypeScript (the parity target)
`src/tui/utils/compactionThresholdParser.ts` — `parseCompactionThreshold(input)`:

```
const MIN_TOKEN_THRESHOLD = 1000;   // line 15
const MIN_PERCENTAGE      = 1;      // line 18
const MAX_PERCENTAGE      = 100;    // line 21

// percentage branch (trailing '%'):
if (isNaN(pct) || pct < MIN_PERCENTAGE || pct > MAX_PERCENTAGE) return undefined;  // line 39
//   → valid: 1..=100 inclusive

// token branch (bare integer):
if (isNaN(tokens) || tokens < MIN_TOKEN_THRESHOLD) return undefined;               // line 46
//   → valid: >= 1000
```

Out-of-range input (`"0%"`, `"101%"`, `"999"`) returns `undefined` → the field is
treated as **unset** and omitted from the saved profile object.

This parser is called **only** from the profile form:
`src/tui/inputHandlers/profileFormModeHandler.ts:163` (`parseCompactionThreshold`).
(The custom-*model* form is a separate feature and does NOT share this parser in TS —
see §3 scope note.)

### Rust (current behavior — the gap)
`codelet/fspec-tui/src/views/model_selector/form.rs:222-237` —
`parse_compaction_trigger(raw) -> (Option<String>, Option<u32>)`:

```rust
if let Some(pct) = trimmed.strip_suffix('%') {
    if let Ok(n) = pct.trim().parse::<u32>() {
        return (Some("percentage".to_string()), Some(n));   // NO 1..=100 range check
    }
    return (None, None);
}
if let Ok(n) = trimmed.parse::<u32>() {
    return (Some("tokens".to_string()), Some(n));           // NO >=1000 minimum
}
(None, None)
```

The profile form reaches this via
`provider_settings/profile_form.rs:149-150` (`build_definition` →
`parse_compaction_trigger`). **No range enforcement.** So Rust silently accepts and
persists `"0%"`, `"101%"`, `"500"` as valid `compactionThreshold` values, diverging
from TS which would drop them as unset.

---

## 2. Required behavior (parity)

In the **profile form save path** (`build_definition`), a compaction-threshold input
must be treated as **unset** (both `compaction_threshold_type` and
`compaction_threshold_value` = `None`, i.e. omitted from the saved
`ProfileDefinition`) when:

| Input | TS result | Required Rust result |
|---|---|---|
| `""` (empty) | undefined | unset (already correct) |
| `"80%"` | `{percentage, 80}` | `{percentage, 80}` |
| `"1%"` | `{percentage, 1}` | `{percentage, 1}` (min inclusive) |
| `"100%"` | `{percentage, 100}` | `{percentage, 100}` (max inclusive) |
| `"0%"` | undefined | **unset** (currently wrongly `{percentage,0}`) |
| `"101%"` | undefined | **unset** (currently wrongly `{percentage,101}`) |
| `"200000"` | `{tokens, 200000}` | `{tokens, 200000}` |
| `"1000"` | `{tokens, 1000}` | `{tokens, 1000}` (min inclusive) |
| `"999"` | undefined | **unset** (currently wrongly `{tokens,999}`) |
| `"abc"` | undefined | unset (already correct) |

Constants to mirror: `MIN_PERCENTAGE = 1`, `MAX_PERCENTAGE = 100`,
`MIN_TOKEN_THRESHOLD = 1000`.

---

## 3. Scope & design constraint (IMPORTANT)

`parse_compaction_trigger` is a **shared** helper also consumed by the
model_selector custom-model form (`model_selector/form.rs`). In TypeScript the
custom-model form does **NOT** use `parseCompactionThreshold` (that parser is
profile-form-only), so blindly adding range checks to the shared Rust parser could
make the Rust custom-model form **stricter than its TS counterpart**.

**Design decision for this card:** enforce the range on the **profile save path
specifically** — e.g. apply a range guard in `profile_form::build_definition` after
calling the shared splitter, or introduce a profile-scoped validated wrapper — so
the model_selector custom-model form behavior is **unchanged**. The implementing
worker must confirm (read TS `customModelForm` handling) whether the custom-model
form should also enforce the range; if TS does NOT, leave it alone and add a
regression note. Either way, the existing model_selector tests must stay green.

---

## 4. Edge cases to cover in tests

- `"1%"` and `"100%"` accepted (inclusive boundaries).
- `"0%"` and `"101%"` rejected → unset.
- `"1000"` accepted; `"999"` rejected → unset.
- Empty / non-numeric → unset (no regression).
- The rejected value still leaves the rest of the profile saveable (baseUrl/apiKey/
  name present) — i.e. an out-of-range threshold does NOT block save, it is just
  omitted (matches TS conditional-spread omission on `undefined`).
- Round-trip: a profile saved with an out-of-range threshold has NO
  `compactionThreshold` key in `~/.fspec/fspec-config.json`.

---

## 5. ACDD constraints

- Strict 100% ACDD: feature file → failing tests (witnessed RED) → impl.
- Offline tests only; no real `~/.fspec`, no env mutation; MockBackend where a
  backend round-trip is asserted.
- Files < 300 LoC; clippy `-D warnings`; cargo fmt clean; NO git; do not touch
  user WIP (main.rs / session_manager.rs).
- Verify `cargo test -p codelet-fspec-tui` (whole crate) stays green — especially
  any model_selector custom-model form tests — to prove the shared parser change
  (if any) did not regress them.
