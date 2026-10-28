# Board navigation and new-agent work-unit assignment

Date: 2026-10-28

Related cards:
- **BUG-280:** Board Enter reassigns the current agent instead of opening the selected story's session.
- **BUG-281:** New Agent and right-edge Shift+Right inherit stale work-unit selection.

## Investigation status

This is a source-code and existing-specification investigation. No production code was changed and no tests were executed as part of the investigation. Reproduction sequences below are derived from the traced code, not a recorded interactive run. File line references identify the implementation inspected on the date above and may move later.

The user reports that entering a story from the board assigns an already-open or resumed agent, and that creating a new agent inherits the current story when the new agent should have no work unit.

## Summary

The TUI conflates three distinct intentions: navigating to a story's session, assigning a story to an existing session, and creating an unassigned session. A legacy global `current_work_unit_id` remains an input to creation even though displayed activity focus is now per-session and authoritative.

The two bugs should be coordinated but need not be marked as blocking one another. BUG-280 owns board-entry routing; BUG-281 owns generic creation and implicit-assignment removal. Both touch creation intent and require consistent specifications.

## BUG-280: Board Enter targets the current agent

### Current path

1. `rust/fspec-tui/src/views/board.rs:190` emits `EnterWorkUnit(selected_id)` for Enter.
2. `rust/fspec-tui/src/app/dispatch_work_unit_entry.rs:17` writes the selected story into the global current-work-unit fields, opens Agent view (or queues mux entry), and queues `AttachWorkUnitToSession`.
3. `rust/fspec-tui/src/app/dispatch_work_unit_binding.rs:34` resolves the current session when handling that action, then calls `backend.set_work_unit_context` for that session.
4. Only when there is no current session does the entry handler create one. That branch directly requests a non-isolated session rather than offering the creation dialog.

There is no initial lookup of the selected story's associated session in this Enter path. A resumed session is simply another current session and can therefore be reassigned.

By contrast, board Shift+Right calls `selected_session` before emitting `OpenAgentView` (`rust/fspec-tui/src/views/board.rs:183`). These two keyboard paths therefore implement different targeting policies.

### Reproduction sequences

- Open agent A on story AUTH-001. Return to the board without closing A. Select unrelated story AUTH-002 and press Enter. The handler attaches AUTH-002 to A rather than preserving A's assignment.
- Resume a previously closed session, return to the board, and enter an unrelated story. The resumed session can be reassigned through the same path.
- With A current and another open agent B associated with the selected story, Enter still follows the current-session attachment path rather than first resolving B.
- With no current agent, Enter creates a non-isolated session directly instead of offering the normal/isolated choice.

### Recommended behavior

- Resolve the selected story's associated open session first and navigate to it without reassigning any other session.
- If no associated open session exists, offer a creation dialog carrying explicit intent for that story.
- Cancellation must not change existing assignments or leave pending story intent for a later generic creation.
- Keep normal versus isolated creation available for story-specific creation.
- When several open sessions share a story, preserve the existing earliest-open-session navigation representative unless a separate product decision changes it.
- Do not silently resume arbitrary disk-only sessions. Explicit resume remains a separate action; closed-session discovery policy should be clarified during specification if required.
- Review mux entry alongside normal board entry rather than allowing pane focus to determine assignment accidentally.

## BUG-281: Generic creation explicitly attaches stale selection

### First attachment route: dialog confirmation

`rust/fspec-tui/src/app/dispatch_create_session_dialog.rs:70` handles confirmation for the shared creation dialog. It reads `current_work_unit_id()` and passes that value as `card` to `run_session_creation`.

`rust/fspec-tui/src/app/session_creation.rs:35` converts a successful creation with `Some(card)` into `WorkUnitSessionCreated`. `rust/fspec-tui/src/app/dispatch_focus.rs:6` then initializes/focuses the new session and attaches that card.

This affects New Agent and Shift+Right at the right edge of the session list because both open the same dialog. It applies to normal and isolated creation. The association is sent to the backend; this is not merely incorrect header text.

### Second attachment route: generic initialization

`rust/fspec-tui/src/app/dispatch_create_session_dialog.rs:99` handles ordinary `SessionCreated` with `initialize_session(session, true)`. At line 148, initialization reads the same global work-unit selection and queues attachment actions.

Removing only the dialog's `card` argument is insufficient: a successful blank creation becomes ordinary `SessionCreated` and can still attach the global selection through this fallback.

### Why the selection becomes stale

- Returning to the board without closing only changes view and saves the draft (`rust/fspec-tui/src/app/dispatch_session_cycle.rs:118`).
- Switching sessions changes the focused session and restores its draft without synchronizing the legacy global story (`rust/fspec-tui/src/app/dispatch_session_cycle.rs:134`).
- The creation dialog title reads the current session's per-session context (`rust/fspec-tui/src/app/dispatch_create_session_dialog.rs:46`), while confirmation reads the global selection. The displayed story and actual assignment can therefore disagree.
- Fresh backend session construction starts with no work-unit context and default focus (`rust/sessions/src/background_session/construction.rs:84`). The unwanted association is introduced afterward by the TUI.

### Reproduction sequences

- Enter AUTH-001 from the board, then choose New Agent and confirm. The new session can receive AUTH-001.
- Enter AUTH-001, move to the last open agent, and press Shift+Right to create another agent. Confirmation uses the same inheritance path.
- Enter AUTH-001, switch to a session displaying another story or no story, then create an agent. The stale global value can still attach AUTH-001.
- Repeat normal creation cases with isolated creation; both use the same captured card.

### Recommended behavior

- New Agent and right-edge Shift+Right creation must explicitly mean an unassigned session.
- Generic session initialization must never infer assignment from board selection or the previously active agent.
- Give the creation request/dialog explicit optional story intent: absent for generic creation, present only for a deliberate story-specific creation action.
- Use that same request context for the dialog title and the eventual attachment, rather than reading different mutable stores at each stage.
- Carry the newly created session ID together with explicit story intent through the asynchronous completion path.
- Attachment operations should receive their target session ID explicitly instead of resolving whichever session is current when a queued action is processed.
- Keep creation acknowledgements and session-observation events idempotent without introducing implicit assignment.
- Do not fix inheritance by clearing another session's legitimate authoritative focus.

## Navigation, close, and resume distinctions

Shift+Right does not always create an agent. Within the session list it navigates; at the right edge it opens creation. From the board it targets the selected story's session when available, otherwise the first open session, and opens creation only when none are open (`rust/fspec-tui/src/app/dispatch_session_cycle.rs:53` and `:91`). Preserve this distinction in tests and descriptions.

Close clears the local global pointer (`rust/fspec-tui/src/app/dispatch_agent_exit.rs:123`), but backend destruction deliberately retains the persistence manifest (`rust/sessions/src/session_manager.rs:1410`). Explicit resume restores that session's own persisted focus and validates the referenced story; missing or done stories are cleared (`rust/sessions/src/activity_focus.rs:95`). Restoring a session's own association is different from assigning it an unrelated story or giving a new session another agent's story.

## Conflicting specifications and existing tests

- `spec/features/enter-key-on-board-bypasses-create-session-dialog-no-isolated-option.feature:15` requires navigating to an attached session, otherwise showing the creation dialog. Its architecture notes reference the former TypeScript implementation, but its behavior is relevant to BUG-280.
- `spec/features/session-work-unit-focus-projection.feature:51` instead specifies entry into a different card through an existing session.
- `rust/fspec-tui/tests/bug230_focus_projection.rs:110` explicitly asserts that entering AUTH-002 reassigns the existing AUTH-001 session. This test preserves the reported board behavior.
- `rust/fspec-tui/tests/bug230_focus_recovery/lifecycle.rs:40` seeds global selection and asserts that dialog creation attaches it, without distinguishing a generic New Agent request from story-specific creation intent.
- Existing cleared-focus display tests guard projection against global fallback, but do not prevent creation from turning a stale global selection into a new authoritative assignment.

Update the behavioral contract before changing these tests. Retain their legitimate protections for authoritative focus, revision ordering, and observation-before-acknowledgement races; replace the ambient-assignment assumptions with explicit request intent.

## Suggested acceptance and regression matrix

| Card | Case | Expected outcome |
| --- | --- | --- |
| BUG-280 | Enter a story associated with a different open agent | Navigate to that agent; current agent remains unchanged |
| BUG-280 | Enter an unassociated story while an unrelated agent is open | Show story-specific creation dialog; do not reassign the open agent |
| BUG-280 | Enter an unassociated story with no agents open | Offer normal/isolated creation; bind only after successful confirmation |
| BUG-280 | Enter a story after resuming an unrelated agent | Preserve resumed agent's own focus |
| BUG-280 | Cancel story-specific creation, then create a generic agent | No assignment changes; new agent has no story |
| BUG-280 | Multiple agents share a story | Navigate using the established representative without changing other agents |
| BUG-281 | New Agent while current agent has a story | New session has no story; original session retains its story |
| BUG-281 | Right-edge Shift+Right with a stale global selection | New session has no story |
| BUG-281 | Generic creation after switching to a cleared or differently assigned session | Neither the old global story nor displayed story is inherited |
| BUG-281 | Normal and isolated generic creation | Both start unassigned |
| Both | Session observed before local creation acknowledgement | No accidental assignment, duplicate attachment, or loss of explicit story intent |
| Both | Session focus changes while creation/attachment completes | Explicit intended target is used, not whichever agent is current |
| Both | Board, Agent, and mux entry points | Same creation-intent policy; unrelated sessions remain unchanged |
| Both | Creation declined, fails, or is cancelled | No reassignment and no pending intent leaked into later creation |
| Both | Explicitly resume a persisted session | Restore only its own valid saved focus; no global-selection fallback |

## Implementation and validation approach

1. Resolve conflicting specifications and define explicit blank-versus-story creation intent.
2. Add focused failing tests for each accepted scenario before production changes.
3. Correct board routing and remove both generic inheritance routes using shared creation logic, not menu-specific patches.
4. Preserve per-session authoritative focus and revision-ordering protections.
5. Run scoped integration tests, compilation, linting, and feature/tag validation. Never use unscoped workspace tests.

Queued attachment targeting and event-ordering cases above are preventive regression recommendations based on the action shape; they were not separately reproduced as runtime races during this investigation.
