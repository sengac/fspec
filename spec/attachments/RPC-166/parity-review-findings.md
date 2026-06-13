# Parity Review — fspec TS→Rust Port (Last Batch: 10 RPC commands)

**Date:** 2026-06-13
**Reviewer:** Claude Code (supervisor) + 5 parallel agents
**Method:** Ran the original TypeScript `fspec` CLI vs the Rust release binary
(`codelet/target/release/fspec`) in paired isolated `/tmp` dirs; diffed stdout,
stderr, exit codes, and resulting file mutations (`jq -S`, byte-diff).

## Commands reviewed (5 add/remove pairs)
| Pair | RPC (add) | RPC (remove) |
|------|-----------|--------------|
| aggregate-to-foundation | RPC-166 | RPC-266 |
| capability | RPC-173 | RPC-269 |
| command-to-foundation | RPC-175 | RPC-270 |
| foundation-bounded-context | RPC-183 | RPC-274 |
| persona | RPC-186 | RPC-277 |

## Agent assignment
- **Build agent** (cargo gatekeeper, serial): all `cargo build/test/clippy`; also owned the aggregate pair.
- 4 parity reviewers: capability, command, bounded-context, persona pairs.

---

## Parity defects found & FIXED

### persona (RPC-186 / RPC-277)
- Missing-foundation stderr: TS prints 3 lines (add) / 2 lines (remove); Rust printed 1. Fixed bridge + core.
- Not-found / no-personas stderr: missing `✗ ` prefix and `". "`→`"\n  "` line split. Fixed `remove_persona.rs` core + bridges.

### capability (RPC-173 / RPC-269)
- Missing-foundation stderr (3-line / 2-line parity) — fixed.
- Malformed `solutionSpace` (missing/null/primitive): TS emits exact V8 TypeError strings + exit 1; Rust auto-initialised and succeeded. Fixed via JS-deref-order replication.
- Non-array `capabilities`: TS `e.every is not a function` exit 1; Rust coerced to `[]`. Fixed.
- remove-capability silent-TypeError path + `Array.join` coercion of available-names list. Fixed.

### command-to-foundation (RPC-175 / RPC-270)
- Missing-foundation auto-create wrote the rich `ensureFoundationFile` default; TS writes a slim default. Fixed (`read_or_init_json` + `foundation_read_default`).
- FOUNDATION.md regeneration missing — see RPC-233 below.

### foundation-bounded-context (RPC-183 / RPC-274)
- Bridge printed a spurious `  Regenerated: spec/FOUNDATION.md` line TS never emits — removed.
- Auto-create slim-default fix (same as command pair).

### aggregate-to-foundation (RPC-166 / RPC-266)
- Auto-create slim default; `eventStorm` falsy handling (raw TypeError parity); `nextItemId` undefined/NaN/float handling; `boundedContextId` linkage coercion; empty `--description ""` key omission. All fixed.

---

## Scope decisions (confirmed with user)

1. **FOUNDATION.md regeneration → PORT RPC-233 (done).** TS regenerates `spec/FOUNDATION.md`
   after aggregate/command/bounded-context mutations (6 of the 10 commands); the Rust port
   stubbed it. **Fully ported:** new `generators/foundation_md*.rs` (split <300 lines),
   `generate-foundation-md` command, CLI bridge, help config, canonical/dispatch wiring, and
   regeneration calls wired into the 6 commands. **Verified byte-for-byte:**
   `generate-foundation-md` on the real `foundation.json` → FOUNDATION.md identical (30592 bytes);
   all 6 regenerating commands → stdout + FOUNDATION.md + foundation.json identical.
   - Documented divergences (consistent with existing Rust conventions): Ajv schema-error text
     not reproduced (validate-foundation-schema still a stub); mermaid validation uses the
     established lightweight `add_diagram` pre-check (full `mermaid.parse()` cannot run in Rust).

2. **Project-wide clippy debt → FIX ALL (done).** Cleared the entire 146-error deny-level
   baseline (`expect_used`, `unwrap_used`, `redundant_clone`, `redundant_closure_for_method_calls`,
   `uninlined_format_args`, `needless_lifetimes`, `needless_collect`, `manual_*`, `ptr_arg`, etc.)
   across ~45 source files + ~18 integration-test files, behaviour-preserving. Both crates now
   clippy-zero (production AND tests).

3. **Malformed-JSON error wording → ACCEPT DIVERGENCE (per user).** Matching V8's `JSON.parse`
   error strings byte-for-byte is infeasible in Rust; exit codes already match.

## Incidental fixes
- Two stale dispatcher regression tests pinned to the now-ported `add-persona` stub → repointed to `add-architecture` (RPC-167).
- `cargo_shape.rs` locked-file-layout manifest bumped 108→109 for the new `generate_foundation_md.rs` bridge.

---

## Final verification (definitive sign-off run)
| Check | Result |
|-------|--------|
| Release build (`codelet-fspec`) | ✅ OK |
| Clippy `codelet-fspec-core --tests` | ✅ ZERO |
| Clippy `codelet-fspec --tests` | ✅ ZERO |
| Tests `codelet-fspec-core` | ✅ 1374 / 0 |
| Tests `codelet-fspec` | ✅ 811 / 0 |
| Independent parity spot-checks | ✅ persona stderr identical; generate-foundation-md FOUNDATION.md byte-identical |

**Status: fully clean.** All identified parity defects fixed; RPC-233 ported with byte parity;
entire clippy baseline cleared; all 2185 tests green.
