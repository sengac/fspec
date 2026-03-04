# REMIND-016 — System-Reminder Priority Elevation Plan

## Problem Statement

Runtime `<system-reminder>` blocks currently do not have a deterministic, enforceable priority model that guarantees they are applied before state-changing actions. This creates avoidable workflow errors and context drift.

## Incidents Observed

1. **Ambiguous board movement interpreted as forward transition**
   - User asked to “move this through the board.”
   - Work unit `RIG-011` was advanced to `testing` without explicit target confirmation.
   - Result: unnecessary state churn and confusion.

2. **Auto-advance command path inconsistency**
   - `auto-advance` behavior in the tool path was inconsistent with command reference semantics.
   - Attempt with ID failed (`too many arguments`), attempt without ID failed (`Work unit undefined`).
   - Result: fallback to manual status changes with increased ambiguity.

## Root Cause Themes

- **No explicit reminder-priority contract** in the runtime decision loop.
- **No deterministic supersedence rules** (especially where reminder text says “supersedes earlier environment reminder”).
- **No mandatory pre-action reminder-consistency checkpoint** before status updates.
- **No required clarification gate** when destination state intent is ambiguous.

## Priority Model (Required)

### Instruction Precedence

1. System-level directives (platform/harness)
2. Developer directives
3. Runtime `<system-reminder>` blocks (authoritative operational context)
4. User requests (interpreted through constraints above)

### Reminder Supersedence Rules

- **Scope keying**: each reminder should map to a scope key (`environment`, `work-unit-context`, `workflow-guardrail`, etc.).
- **Latest-wins within scope** by default.
- If reminder contains explicit supersedence language (e.g., “supersedes earlier ...”), older same-scope reminders are hard-invalidated.
- If two active reminders conflict and no supersedence marker exists, action is blocked and clarification is required.

## Behavioral Guardrails (Must Implement)

1. **Pre-action reminder-consistency check** before any state-changing command (`update-work-unit-status`, `auto-advance`, board move actions).
2. **Ambiguity confirmation gate**:
   - Agent must echo: current work unit, current state, intended target state.
   - If user intent does not specify target state explicitly, ask for confirmation before mutating state.
3. **Reminder ledger in memory**:
   - Keep normalized active reminder set with scope + timestamp + supersedes metadata.
4. **Action audit trail**:
   - Log which reminder constraints were applied before each state mutation.

## Acceptance Criteria

1. All state-changing actions evaluate active `<system-reminder>` constraints first.
2. “Supersedes earlier ...” reminders deterministically replace old same-scope reminder context.
3. Ambiguous user requests cannot trigger status transitions without explicit target confirmation.
4. Conflicting reminder context blocks transitions and requests clarification.
5. Audit output shows the reminder constraints used for each transition decision.

## Recommended Test Scenarios

1. **Supersedence enforcement**
   - Given two environment reminders where the second supersedes the first
   - When a transition command runs
   - Then only the second reminder governs decisions

2. **Ambiguous transition blocked**
   - Given user request “move through board” and current status `implementing`
   - When no target status is explicitly provided
   - Then transition is not executed and confirmation is requested

3. **Conflict detection**
   - Given active reminders with conflicting work unit context
   - When transition is attempted
   - Then action is blocked and conflict is surfaced

4. **Auto-advance argument consistency**
   - Given tool-path invocation of `auto-advance`
   - When called with and without ID
   - Then behavior is deterministic, documented, and validated

## Rollout Notes

- Apply this policy to **all** `<system-reminder>` blocks, not only fspec-specific reminders.
- Keep behavior provider/agent-agnostic (Codex, Claude, Cursor, etc.).
- Document the policy in bootstrap/help output where reminder behavior is explained.
