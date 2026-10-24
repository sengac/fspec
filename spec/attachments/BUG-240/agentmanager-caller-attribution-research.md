# BUG-240: AgentManager caller attribution is not child assignment

## Observed incident

On 2026-10-22, while coordinating BUG-232 through BUG-239, the calling
assistant supplied `work_unit_id: "BUG-232"` to `AgentManager(action="spawn")`
for the DAG worker, and `work_unit_id: "BUG-234"` for the timing worker.
The user pointed out that this changes the **calling session's** activity
focus, not the spawned session's work-unit focus. The assistant accepted
the correction. This is an observed agent/tool-contract usability failure;
it is not evidence that admission incorrectly attributes the call.

The spawn results were worker IDs `8ed15b87-b3c8-4556-915e-26a5c21ec769`
and `edc99a46-82fe-4d51-b8c3-b0cef6e5d91d`, respectively. Role text
assigned business tasks to these workers. Role text and the harness
attribution field are distinct concepts.

## Verified implementation

- `rust/tools/src/tool_attribution.rs:102`: the schema decorator adds a
  required root `work_unit_id`. Its description says "Required harness
  attribution", references the "invoking project", and excludes business
  targets and permission changes. It does not explicitly say "calling
  session, never the spawned/target session" or give an AgentManager example.
- `rust/tools/src/tool_admission.rs:30`: `split_arguments` removes the root
  attribution field from the business argument object.
- `rust/tools/src/tool_admission.rs:83`: admission uses `self.scope`, the
  invoking context's UUID, before delegating `tool.call(business)`.
- `rust/sessions/src/focus_reconciliation.rs:14`: `admit_tool_invocation`
  resolves the given session, validates the card in its project, and applies
  its focus before execution. `NONE` explicitly clears; it does not retain.
- `rust/tools/src/agent_manager/types.rs:48`: `Spawn` has only an optional
  role, not a child work-unit assignment parameter.
- `rust/agent-loop/src/agent_manager_handler.rs:120`: spawning creates a
  new session, registers supervision and sets role/model limits; the root
  harness field is not passed here as a child assignment.
- `docs/research/session-work-unit-focus.md`: existing architecture
  explicitly validates and strips attribution before business delegation.
- `spec/features/session-work-unit-focus.feature:22`: existing acceptance
  criteria require isolated, project-validated session focus and preserved
  authorization/worktree bindings.

## Root-cause assessment and limits

The assistant conflated invocation attribution with the work being delegated.
The current field description is technically consistent with caller admission,
but leaves the caller-versus-subject distinction implicit for management tools.
Source inspection supports the user's explanation. No runtime focus-event
capture was performed; do not claim that the child inherited a particular focus
or that the attribution backend is defective.

## Proposed scope for subsequent ACDD work

1. Explicitly explain that root `work_unit_id` changes the caller's focus,
   including for spawn/message/status/profile calls about another session.
2. Explain that assigning a worker task in role/message text does not set
   that worker's authoritative focus; its own admitted calls must attribute
   its work. Investigate a separately named child initialization argument
   only if the product owner requests that capability.
3. Specify a spawn regression: caller attributes orchestration to card A,
   worker is tasked with card B, caller remains A, and that harness field
   never silently assigns B to the child.
4. Test emitted schema descriptions and real admission/session isolation,
   including `NONE`, invalid cards, targeted management calls and retained
   authorization. Preserve required validation and business-field stripping.
5. Verify both shared and any legacy NAPI management routes before claiming
   parity. This research inspected the shared spawn path only.

## Status

Research-only backlog card requested by the user. No production changes,
new executable acceptance criteria or fix claims are included. The original
BUG-232 through BUG-239 goal remains separate.
