# PROV-147 — Codex-only compatibility boundary

## Goal and evidence

Establish a safe implementation boundary for the Codex streaming parity epic.
The audit compared fspec `38a95522` with upstream Codex `bb64b89` on 2026-10-15.
Findings are static-source observations, not newly reproduced live failures.
Upstream paths below are relative to https://github.com/openai/codex at that revision;
the temporary `/tmp/codex` checkout is a convenience, not a durable dependency.

Current entry points: `rust/providers/src/codex/mod.rs` and `refreshing_client.rs`.
The Codex provider currently uses shared rig Responses types and generic agent history.
Those shared paths also serve other providers; changing their defaults is not Codex isolation.

## Mandatory scope contract for every epic card

- Own new policy, wire types, request building, parsing, history and transport under
  `rust/providers/src/codex/` or an explicitly Codex-named adapter/module.
- Select the adapter by resolved provider identity, never model-name substring or a global flag.
- Shared plumbing changes are permitted only as narrow, explicit, default-off Codex integration
  seams. They must not change non-Codex execution, serialization, retry, history or UI behavior.
- Do not globally change `rust/patches/rig-core/src/providers/openai/`, generic stream loops,
  history stripping, thinking defaults, or other providers to implement these cards.
- Preserve behavior at the audited baseline, including existing BUG-208 changes. Reverting or
  broadening earlier shared changes is outside this epic and requires separate approval.
- Ask before a major refactor; the card authorizes planning, not an unrestricted rewrite.

## Acceptance criteria

1. Selecting the Codex provider activates the new boundary; selecting any other provider does not.
2. Identical model names configured on different providers cannot activate Codex behavior accidentally.
3. Existing text, reasoning-summary, function-call and usage behavior remains functional for Codex.
4. Non-Codex request/event snapshots and history/retry behavior remain unchanged by this epic.
5. Every later card documents any shared seam and includes a non-Codex regression control.

## Implementation and test plan

First produce a small architecture decision describing adapter ownership and how typed Codex
events/history reach the agent and frontend without changing default generic behavior.
Prefer composition and narrow interfaces over copying the entire agent implementation.
Keep new files below 300 lines; splitting existing large shared files is not authorized here.

Before production edits: Gherkin scenarios, then failing integration tests, then implementation.
Use real local transport fixtures and shared helpers from `rust/test-helpers/`; no real credentials
or network access are required. Map scenarios 1:1 to tests. Run package-and-target-scoped Cargo
tests, compilation and lint checks, plus feature/tag validation. Use snapshot/property testing
where useful for wire formats. Live parity tests must be explicitly opted into and redacted.

## Ordering and exclusions

This is the prerequisite for PROV-148 through PROV-158. Correctness comes before optional
WebSockets or native tool dialects. Existing bugs outside Codex remain separate work.
No claim of complete upstream parity is valid until the individual capability tests pass.
