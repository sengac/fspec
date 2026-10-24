# BUG-212: inactive Codex state vetoes OpenAI API protocol compaction

## Runtime evidence

Screenshot: `/Users/rquast/Desktop/Screenshot 2026-10-21 at 11.55.39 am.png`.
The selected model is `qwen3.8-flash-next-iq3_s`. The error is a compaction
**commit** failure, not a Codex inference request or builder depth failure.

`~/.fspec/logs/fspec-combined.log.2026-10-21` records:

- Lines 144–153, 01:02:35Z: session `93c2deaa-71da-4262-9f84-8f8e1a726d0d`
  is initially created with `codex/gpt-6.1-sol` in `/Users/rquast/uq/Bpd-7.7.5`.
- Lines 162–169, 01:02:40Z: the same session switches to
  `openai:Strada/qwen3.8-flash-next-iq3_s`; model resolution explicitly bridges
  the OpenAI API profile to its compatible endpoint. This is an API protocol
  selection for a Qwen model, not an OpenAI vendor selection or Codex routing.
- Line 407, 01:49:17Z: the loop still selects the Qwen model.
- Lines 475–477, 01:52:17Z (11:52:17 local): the builder finishes at depth 15/20,
  but commit fails with `Codex checkpoint lineage mismatch: generation must
  advance with a selected model`.

No credentials or message contents are copied into this attachment.

## Root cause

`ProviderSession` retains registered extensions after a model switch.
`sessions/model_switch.rs::persist_selection` resets provider state, advancing
the retained Codex history generation independently of the durable DAG boundary.
The compaction lifecycle broadcasts `prepare_generation`,
`install_committed_generation`, and `validate_committed_restore` to **every**
registered extension. The OpenAI API transport therefore calls Codex lineage checks even though its
selected serializer and provider identity are correct. A first durable generation
of 1 can collide with the inactive Codex generation of 1 or higher.

The error's reference to a selected model is generic: `CodexHistory` uses the same
error for a non-advancing generation and an empty model. The logs do not establish
that the selected OpenAI API model identifier was missing.

The broadcast behavior and durable compaction integration originate in
`b746189b` (`refactor: redo compaction`). BUG-211 changes builder exhaustion and
fresh-input retries, not checkpoint extension ownership.

## Fix constraints

- Scope new durable checkpoint preparation to the selected **base provider**,
  not its profile-qualified label. Installation, reconciliation and restore also
  include extensions explicitly owning metadata in the persisted boundary.
- Keep inactive extensions available for provider switching, but never use their
  generations or metadata to certify another provider's boundary.
- Preserve exact profile identity, selected serializer admission, durable
  publication and fail-closed reconciliation.
- Do not weaken Codex checkpoint lineage checks or rewrite live user session data.

## Acceptance and validation

Six scenarios across two features cover switched OpenAI API commits, restore,
publication failure/reconciliation, selected Codex lineage rejection, full
SessionManager replay/cold-process resume, and direct restore atomicity. Tests
use shared temporary-directory helpers, real persistence and offline selected
serializers; no live session was rewritten.

### Implementation and architectural boundary

- `ProviderSession::for_selected_provider` scopes new checkpoint preparation and
  installation without removing retained extensions needed for future switches.
- `ProviderSession::for_compaction_checkpoint` selects the active transport plus
  extensions explicitly claiming ownership of persisted checkpoint metadata.
  Ownership is declared by the extension, not by Codex-specific logic in shared
  context handling. The Codex extension owns `codexContinuation`.
- The shared DAG remains provider-neutral and can preserve work from several
  providers' models/agents. OpenAI API names a compatible protocol; Codex is a
  separate integration, not a prerequisite for that protocol.
- Committed restore preflights authoritative envelopes before mutating checkpoint,
  messages or usage. It restores scoped continuation state as part of boundary
  installation. SessionManager no longer follows that with an unscoped restore;
  ordinary uncommitted replay retains its prior validation path.
- Exact profile identity, selected serialization/admission, publication barriers
  and Codex lineage checks remain enforced.

### Red-to-green evidence

- `/tmp/bug212-red.log`: three switched-provider regressions failed; the selected
  Codex lineage guard passed. Restore reported a missing required Codex checkpoint.
- `/tmp/bug212-replay-red.log`: full replay changed inactive Codex state from
  `(2, None)` to `(3, None)`, exposing the generic post-boundary restore broadcast.
- `/tmp/bug212-legacy-red.log`: excluding every inactive extension broke four
  existing corrupt-owned-checkpoint tests. Explicit metadata ownership preserves
  those checks without requiring inactive metadata for newly created boundaries.
- `/tmp/bug212-direct-red.log`: direct restore installed checkpoint state before
  rejecting a future retained generation. Moving preflight before installation
  makes the failure atomic and keeps ordinary inference blocked.
- `/tmp/bug212-full-green2.log`: 34 distinct tests passed: new CLI scenarios (4),
  new replay/atomicity scenarios (2), session restore atomicity (4), restore
  lifecycle (8), pending restore (1), profile identity (4), provider identity (7),
  and runtime commit (4). Cold subprocess echoes are not additional tests.
- `/tmp/bug212-final-validation.log`: 22 additional distinct tests passed:
  provider idempotent boundary (3), restore preflight (2), Codex history (2),
  Codex reload (3), and all BUG-211 regressions (8 + 2 + 2).

### Final quality checks

All Cargo commands were scoped to packages/targets, used `--profile ci-test`, and
disabled debug information with `CARGO_PROFILE_CI_TEST_DEBUG=0`,
`CARGO_PROFILE_DEV_DEBUG=0`, and `CARGO_PROFILE_TEST_DEBUG=0`.

- `cargo check --profile ci-test` and `cargo clippy --profile ci-test` passed for
  the providers, CLI, sessions and agent-loop libraries.
- Scoped Clippy passed for both new integration targets. Existing unused helper
  warnings remain in reused fixtures; there were no compile or lint errors.
- Both feature coverage audits passed (12/12 and 9/9 referenced files found).
  The replay sidecar's cached scenario count was corrected to two (100% covered).
- Both feature files passed Gherkin syntax validation. Final formatting, tag and
  workflow checks are recorded in the completion section below.
- `git diff --check` passed. These are scoped results, not a workspace-wide test
  claim. No unscoped Cargo test or live provider request was needed.

## Completion

- Final scoped result: **56 distinct passing tests**, including all six BUG-212
  scenarios and twelve BUG-211 regressions. Repeated cold-process output is
  excluded from the count.
- Rust formatting checks passed for all seven changed/new Rust files. Both
  features were formatted with `fspec format` and passed Gherkin validation.
- Isolated tag validation against the repository's registered tags and work units
  passed for both final features. The replay feature needed its registered
  `@session` component tag in addition to the `@session-management` feature-group
  tag; that omission was corrected before completion.
- Both features carry `@done`. Workflow transition to validating passed the
  scenario/test/implementation coverage and step-comment gates. Final review also
  corrected one step comment to say OpenAI API and aligned the card's ownership
  rule with the persisted-metadata safeguards.
- The attached research, implementation, scenarios and coverage are ready for
  review. Changes remain uncommitted; no live user session data was changed.
- BUG-212 completed the validating-to-done workflow transition successfully.
