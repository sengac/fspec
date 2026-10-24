# BUG-254: Mandatory persistence prevents Windows background inference

## Review context

- Reviewed commit: `144653b0dc12ca096e835e86b620b109b29019bf` (`more fixes`).
- Review priority: P1 — background prompts cannot proceed on Windows.
- Evidence: static inspection of platform branches and the input-admission path; not reproduced on Windows during review.

## Problem and impact

Every recovery-record promotion synchronizes its containing directory. The
non-Unix implementation always returns an unsupported-operation error. User
input now must complete persistence before inference is allowed, so a Windows
background session cannot pass the new gate even with an otherwise writable
data directory and valid provider configuration.

## Source evidence

Locations refer to the reviewed commit:

- `rust/agent-loop/src/persistence_retry_io.rs:94` calls directory synchronization after promoting a staged record.
- `rust/agent-loop/src/persistence_retry_io.rs:116` selects an unconditional error for non-Unix targets.
- `rust/agent-loop/src/user_persistence.rs:13` submits structured user input through the persistence path.
- `rust/agent-loop/src/persistence_retry.rs:230` submits through the recovery spool before committing originals.
- `rust/agent-loop/src/agent_loop.rs:393` persists each user input before running inference.
- `rust/agent-loop/src/agent_loop.rs:408` skips the turn when that persistence fails.
- `rust/core/src/persistence/messages/durability.rs:61` contains a second non-Unix durability rejection that must be considered for an end-to-end fix.

Previously, the agent loop logged user-persistence failures and continued. The
new refusal is appropriate for real storage failures, but makes the missing
platform implementation a blocker for all Windows turns.

## Reproduction to automate

1. On Windows, use a writable temporary data root and an existing valid session.
2. Submit a normal text prompt to the background agent loop.
3. Observe recovery promotion fail at the non-Unix directory barrier.
4. Verify the loop reports persistence recovery is required and never dispatches the provider request.

Expected: supported Windows storage can durably admit the prompt and run its
provider request; genuine persistence failures still prevent inference.

## Suggested repair direction

Define and implement the supported Windows durability and atomic-publication
contract throughout recovery, message, and manifest storage. Account for the
workspace's prohibition on unsafe code when choosing a platform abstraction.
Do not fix this by ignoring synchronization failures or restoring inference
after unsuccessful persistence.

## Acceptance and validation guidance

- Exercise user admission, journal append, recovery retry, and cold restore on Windows.
- Verify the next provider request runs after successful durable admission.
- Verify failed admission still prevents publication and dispatch.
- Test retry idempotence and crash boundaries under the supported platform contract.
- A Unix-only test run does not validate the affected conditional branch.

Related specifications: `spec/features/persistence-user-admission.feature`,
`spec/features/durable-message-before-manifest.feature`, and
`spec/features/persistence-journal-promotion.feature`.
