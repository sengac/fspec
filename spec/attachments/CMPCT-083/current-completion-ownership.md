# Current completion ownership

This pass preserves the existing staged and unstaged compaction work and finishes
the 27 cards still active at its start. Board changes are coordinator-only; no
commits or staging are requested.

| Owner session | Exclusive source ownership |
| --- | --- |
| `034833d1` | `rust/core/**` |
| `6f5e4aff` | `rust/providers/**` |
| `35fe25d6` | `rust/cli/**` |
| `250c9ab9` | `rust/agent-loop/**`, `rust/tools/**` |
| `f442356f` | `rust/sessions/**` except gate tests, `rust/fspec-tui/**`, `rust/rpc-types/**` |
| `e11304ce` | `rust/patches/rig-core/**` |
| `2ccc6e49` | `rust/sessions/tests/cmpct083*` and the gate audit |
| `25d1be63` | Independent read-only acceptance/traceability audit |

Each source owner maintains its own target-specific feature files and coverage
sidecars. Cross-owner production edits go through the coordinator. Existing
handoffs are useful context, not substitutes for current test results.

## Plan

1. Audit retained implementations, acceptance criteria, and unfinished evidence.
2. Finish genuine behavioral gaps using specification, failing test, then code.
3. Run scoped behavioral targets, compilation, Clippy, formatting, and Gherkin
   validation; reconcile independently audited scenario/test/source mappings.
4. Verify cold-process and no-fallback gates against the integrated source.
5. Attach evidence and advance cards through remaining states in dependency
   order, without temporal-validation or coverage bypasses.

The initially active cards are CMPCT-052, 054, 055, 056, 057, 058, 059, 060,
061, 062, 064, 065, 066, 067, 068, 069, 070, 071, 072, 074, 075, 078,
079, 080, 081, 082, and 083. This document records ownership, not completion.

## Current integration mailbox

- Core's fresh fifteen-target test, check and Clippy runs pass. Its strict
  traceability refresh initially detected stale function spans and now passes
  in `/tmp/cmpct-core-supervised-034833d1-traceability-final2.log`.
- The fresh loop batch exposes five failed targets because the shared selected
  OpenAI fixture omits usage, now correctly rejected by Rig. Loop owns supplying
  explicit measured usage in that scripted response, not weakening admission.
- CLI's genuine current behavioral REDs are
  `/tmp/cmpct-cli-supervised-fill-red.log` and
  `/tmp/cmpct-cli-supervised-terminal-red.log`. The latter needs loop registration
  of the API documented in `CMPCT-059/cli-supervised-api-state.md`.
- Fresh sessions restore tests pass. The remaining TUI spend assertion remains
  unchanged and fails pending the CLI aggregation repair.
- Gate's first full run reaches Codex physical request assertions, not a registry
  limit failure. Gate is replacing string field counting with exact native
  function-call/result pairing, preserving the acceptance requirement.
- Independent audit output is `/tmp/cmpct-independent-current-summary.json` and
  `/tmp/cmpct-independent-audit.json`. Missing sidecars, steps, implementation
  mappings and stale ranges are owner work, not grounds for skipping gates.
- Global feature syntax passes all 1944 files. Global tag validation retains the
  unrelated baseline of 526 violations; owned features require scoped success.

These are current intermediate results, not completion claims. Busy agent
messages are not queued; this mailbox preserves coordination until delivery.
