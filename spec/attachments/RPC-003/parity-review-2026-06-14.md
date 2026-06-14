# Parity & ACDD Review — Rust CLI Port (last batch of 10 RPC cards)

**Date:** 2026-06-14
**Reviewer:** Claude Code (5-agent parity analysis: 1 build/test agent + 4 parity analysts)
**Binaries compared:** TypeScript `fspec` v0.9.3 (PATH) vs Rust `codelet/target/release/fspec`
**Method:** identical fixtures in `/tmp`, diff stdout/stderr + exit codes; `cargo` build/test/clippy isolated to a single dedicated agent (all others forbidden from running cargo).

## Work units reviewed
| ID | Command | Result |
|----|---------|--------|
| RPC-236 | generate-tags-md | ✅ parity (valid → byte-identical TAGS.md; invalid → identical) |
| RPC-293 | retag | ✅ parity |
| RPC-296 | search-implementation | ✅ parity (text + json) |
| RPC-297 | search-scenarios | ✅ parity (literal + regex + json) |
| RPC-311 | unlink-coverage | ✅ parity |
| RPC-320 | validate | ✅ command logic; parser-text deltas → RPC-329 |
| RPC-321 | validate-foundation-schema | ✅ parity after fixes |
| RPC-322 | validate-hooks | ✅ parity (matches TS exactly) |
| RPC-324 | validate-tags | ✅ full parity |
| RPC-325 | validate-work-units | ✅ parity after fix |

## HEADLINE FIX — real JSON Schema validator (DRY/SOLID)

The Rust port had **hand-rolled "draft-07 subset" validators** in
`generators/foundation_schema.rs` and `generators/tags_schema.rs` — two
near-duplicate recursive walkers that only understood the handful of keywords
and **two hard-coded regexes** present in today's schemas (everything else was
silently "permissive"). This is exactly the correctness hole the TS side avoids
by using **Ajv**.

**Resolution:**
- Added the **`jsonschema`** crate (v0.46) — the de-facto-standard, spec-compliant
  Rust JSON Schema validator (4M+ downloads/month; Tauri & Apollo Router use it;
  75–645× faster than valico/jsonschema_valid). The Rust analogue of Ajv.
- Created ONE shared module `fspec-core/src/validators/json_schema.rs`
  (`validate_against_schema`) — the single validation engine. Both
  `foundation_schema.rs` and `tags_schema.rs` are now thin wrappers delegating
  to it (DRY). No duplicate traversal logic remains.
- Ajv error-string parity preserved: structured `ValidationErrorKind` → exact
  Ajv message text. Two parity adjustments:
  - **Original `pattern` text** recovered from the schema via `schema_path()`
    (the crate normalises `\d`→`[0-9]`; Ajv echoes the literal — now matched).
  - **Ajv error ordering** reproduced via a stable sort by instance-path depth
    (root keywords before per-property errors).
- `draft7` + `should_validate_formats(true)` mirrors Ajv + `ajv-formats`
  (`uri`, `date-time`).
- Cards touching the shared validator were reopened to `specifying` and walked
  back through the full ACDD lifecycle: **RPC-321, RPC-236**, plus the two
  already-`done` consumers **RPC-233 (generate-foundation-md)** and
  **RPC-312 (update-foundation)**.

### Verified end-to-end against the final binary
- Real `spec/foundation.json` + `spec/tags.json` validate ✅ in both; generated
  `FOUNDATION.md` and `TAGS.md` are **byte-identical** TS vs Rust.
- Invalid foundation multi-error output is now **byte-identical** (ordering +
  `\d` pattern text): `#/required → #/additionalProperties → /version → /personas/0`.

## Other fixes
- **RPC-325 validate-work-units:** Rust crashed (`Cannot convert undefined or
  null to object`) on `workUnits: []` where TS reports valid. Replaced the
  over-strict `as_object`-or-throw with faithful JS `Object.entries` semantics
  (`js_object_entries` / `is_js_object_like`): null/undefined → throw (parity),
  arrays/strings/scalars → coerced entries. New integration test added.

## Accepted, documented deviations (NOT defects)
- **oneOf cascade (invalid eventStorm items):** Rust emits a clean 1-line
  `must match exactly one schema in oneOf` per item; TS/Ajv emits a verbose
  per-branch cascade. Same validity, exit code, and items flagged — Rust output
  is strictly cleaner.
- **Malformed-JSON wording:** serde_json vs V8 phrasing differs; identical
  prefix + line/column + exit code. Same class as RPC-329.
- **validate (RPC-320) parser-text:** the lenient `gherkin` crate produces
  different raw error text than `@cucumber/gherkin` (and the unescaped-triple-
  quote content heuristic is masked when that parser errors first). Command
  logic is a faithful port; the raw parser-text divergence is tracked by
  **RPC-329**.
- **validate-hooks (RPC-322):** TS intentionally discards the result (its
  documented "Framing A" broken-CLI pattern); Rust matches TS byte-for-byte.

## Final verification
- `cargo test -p codelet-fspec-core`: 659 lib + all integration suites + 13/13
  validate-work-units, 0 failures.
- `cargo clippy -p codelet-fspec-core --all-targets`: clean (also fixed a
  pre-existing test-helper `expect_used` lint in validate.rs).
- `cargo build --release -p codelet-fspec`: success.

---

## Group B re-verification addendum (2026-06-14, post-rebuild)

Re-ran the Group B (RPC-324 validate-tags / RPC-325 validate-work-units)
discrepancies against the **freshly-built** release binary
(`codelet/target/release/fspec`, mtime Jun 14 23:13) to separate stale findings
from live defects.

| # | Case | Status | Action |
|---|------|--------|--------|
| DISC-1 | `validate-work-units --fix` | **CONFIRMED DEFECT (HIGH)** | Reopen RPC-325 |
| DISC-2 | `workUnits: []` | **RESOLVED (stale binary in report)** | none |
| DISC-3 | `workUnits: "nope"` | **PARTIAL (text-only, LOW/MED)** | document / optional fix |
| DISC-4 | malformed JSON wording | **DOCUMENTED divergence (LOW)** | none (RPC-329 class) |

### DISC-1 — `--fix` is accepted as a no-op by Rust but rejected by TS (HIGH, confirmed defect)
- TS:   `validate-work-units --fix` → `error: unknown option '--fix'`, **exit 1**.
- RUST: `validate-work-units --fix` → `✓ All work units are valid`, **exit 0**.
- `--bogus` is rejected by BOTH (exit 1). Only `--fix` leaks through on Rust.
- ROOT CAUSE: Rust validates runtime flags against the help-config `OPTS` list.
  `--fix` is listed in `OPTS` (required so `--help` stays byte-identical to TS,
  whose rich help also lists `--fix`). Because it is "known" to the help config,
  the runtime arg-validator accepts it as a no-op flag instead of rejecting it.
- VIOLATES **RPC-325 rule [9]**: "clap subcommand exposes NO functional flags
  (TS Commander registration declares none)". `--fix` is documented-only; it must
  NOT be an accepted runtime flag.
- FIX: decouple help-config OPTS (documentation/`--help` rendering) from the
  runtime accepted-flag set for this command. Keep `--fix` in `--help` output
  (byte parity with TS) but reject `--fix` at runtime exactly like any unknown
  option (`error: unknown option '--fix'`, exit 1).

### DISC-2 — RESOLVED
Current binary: `workUnits: []` → BOTH print `✓ All work units are valid`, exit 0.
The Group B analyst tested a pre-`Object.entries`-fix binary. No defect.

### DISC-3 — text-only residual (LOW/MED)
`workUnits: "nope"`: error COUNT now matches (9 vs 9 — both iterate the string).
Residual text divergence from JS coercion:
- TS:   `Invalid status value for 0: undefined` / `status 'undefined' ... states.undefined`
- RUST: `Invalid status value for 0: `        / `status '' ... states.`
TS prints the literal string `undefined` (JS `obj.status` on a char), Rust prints
empty. Faithful-coercion fix is small but both outputs are acknowledged garbage and
there is no acceptance criterion for malformed `workUnits` container types.

### DISC-4 — DOCUMENTED (LOW)
`{` (malformed): TS `Expected property name or '}' ... position 1 (line 1 column 2)`
vs Rust `EOF while parsing an object at line 1 column 1`. Identical wrapper text,
exit 1. serde_json vs V8 — same class as RPC-329. No action.

### validate-tags (RPC-324) — FULL PARITY, no action.

### DISC-1 — RESOLVED (2026-06-14)
Root cause: `Mode::ValidateWorkUnits { fix: bool }` in `codelet/fspec/src/main.rs`
declared a clap `--fix` flag, so clap ACCEPTED it (no-op) instead of rejecting it.
Fix: clap variant now declares NO fields (`ValidateWorkUnits {}`); bridge `CliArgs`
is empty and passes `json!({})`. The help config is unchanged, so `--fix` still
renders in `--help` (byte-parity with TS rich help) but is now rejected at runtime:
`error: unknown option '--fix'`, exit 1 — matching TS Commander. Satisfies RPC-325
rule [9]. Verified: `cargo build --release` ok; `cargo test -p codelet-fspec` =
965 passed / 0 failed / 52 ignored; clippy clean; runtime `--fix` → exit 1, `--help`
→ exit 0 with `--fix` present. New scenario + test added (RPC-325 reopened →
specifying → testing → implementing → validating → done).
