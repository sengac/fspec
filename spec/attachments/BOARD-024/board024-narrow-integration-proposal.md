# BOARD-024: narrow production-integration proposal awaiting approval

## Recommendation

Do not implicitly turn the chooser into a project-wide transport/Action refactor.
The first proposal listed the strict 300-line prerequisite; independent review
correctly identified that as much broader than the requested UX. This supplement
offers a bounded alternative for explicit approval. It grants itself no exception
and preserves the original proposal and evidence unchanged.

## Proposed integration-only exception

Authorize only the following wiring in six already oversized files, with all
new behavior, new types, rendering and tests in modules below 300 lines:

| Existing file | Proposed bounded edit |
| --- | --- |
| rust/fspec-tui/src/components/mod.rs | Register one dialog module and one typed Action payload; place its command/result variants in the new module, not this enum |
| rust/fspec-tui/src/views/navigator_mux_events.rs | Replace only the selected-card Enter emission with chooser action carrying original card ID and Mux origin |
| rust/fspec-tui/src/transport/mod.rs | One full-detail read signature/default unsupported error and one typed BackendError case |
| rust/fspec-tui/src/transport/embedded.rs | Thin delegation to the shared full-detail RPC with typed error mapping |
| rust/fspec-tui/src/transport/websocket.rs | Same thin delegation, preserving existing disconnected-client guarding |
| rust/rpc/src/lib.rs | Register the new reader module, append a detail service declaration, and delegate the handler to that module |

These are proposed registrations/delegations, not permission to reorganize the
existing functions or move unrelated behavior. Review actual diff sizes before
accepting any scope expansion. No change to rpc-types/WorkUnitInfo, the tarpc
envelope/pump, or service-wide business behavior is proposed.

Additional small integration points already below 300 lines are board.rs's Enter
emission, app/mod.rs's module registration, the now-coordinated single capability
branch, footer/help hint text, and dispatch_viewer.rs failure feedback necessary
to keep details visible. No edits to dispatch_work_unit_entry.rs are proposed.

## New modules and responsibilities

- Dialog command/result types, captured card/origin and request generation.
- Chooser interaction and rendering using canonical dialog conventions.
- Details state/history, keyboard/mouse routing and resize-safe geometry.
- Complete canonical-record projection with readable sections and additional
  fields/history; terminal sanitization only at display boundaries.
- Strict daemon-workspace detail reader and attachment availability checks in a
  new RPC helper, without client-local filesystem assumptions or store writes.
- App dispatch helper for read tasks, generation-checked completion and retained
  contextual errors. Reuse exact Action::OpenAttachment paths and existing viewer
  behavior; surface unavailable/browser/path failures without dismissing details.

The full-card capability returns a JSON string, not serde_json::Value across
bincode. Resolve the current watcher workspace; retain every stored card field.
Any ancillary attachment availability data must not replace/filter the original
record. Invalid JSON, missing/malformed cards and non-file attachments are errors.
Existing lexical path rejection is preserved; this is not a security redesign.

## If an oversized-file exception is declined

Production remains paused. A strict no-file-over-300 solution needs a separately
authorized preparatory refactor with its own card, behavioral/contract specs and
review. Mechanical candidates are the component trait/event definitions, backend
constructors/reconnect helpers, RPC service state/domain helper bodies, and mux
event helpers. However, those extracts alone do not make the single large Action
enum, backend trait/impls or tarpc service declaration satisfy the limit.

Those declarations cannot honestly be solved by merely moving the same oversized
block into another file or assuming include! fragments compose under attribute
macros. A stricter decomposition may affect generated service/public API structure
and requires a concrete compatibility design before approval. No wire-protocol
redesign is necessary for the feature-only path, and none is authorized here.

## Exact outstanding decision

Approve the six-file integration-only exception above, **or** authorize a
separately tracked preparatory refactor design/scope before production work.
Neither option is approved yet. Shared dispatch ownership is already resolved and
must not be requested again as though it remained a blocker.

After approval, continue testing -> implementing -> GREEN/regression/quality ->
review/evidence -> canonical Done, never inferring completion from test mappings.
