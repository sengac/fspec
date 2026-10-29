# BOARD-024: integration prerequisites and refactor approval proposal

## Decision received

The user selected **Wait for coordination** for `dispatch_capability.rs` and
**Require refactor proposal** rather than approving additions to oversized files.
No production code or shared dispatch edits have been made for BOARD-024.

## Required integration points

Existing line counts recorded with scoped `wc -l` during discovery:

| Existing file | Lines | Needed for the feature |
| --- | ---: | --- |
| rust/fspec-tui/src/components/mod.rs | 1492 | Register dialogs and new typed actions |
| rust/fspec-tui/src/views/navigator_mux_events.rs | 409 | Emit chooser action instead of MuxEnterWorkUnit |
| rust/fspec-tui/src/transport/mod.rs | 1148 | Dedicated full-card read and typed backend error |
| rust/fspec-tui/src/transport/embedded.rs | 1222 | Delegate full-card read to shared RPC |
| rust/fspec-tui/src/transport/websocket.rs | 1820 | Same delegate with disconnection handling |
| rust/rpc/src/lib.rs | 2668 | Declare and implement shared full-card RPC |

`views/board.rs` is 286 lines and `app/mod.rs` is 195 lines. They can accept
small wiring changes without exceeding the limit. New dialog, rendering,
interaction, reader, dispatch and integration-test files must stay below 300.

## Proposed separate preparatory refactor

Approval is requested before creating or implementing this broader scope:

1. Create a separately tracked preparatory refactor card, with its own Markdown
   design, specifications, behavioral/contract tests and validation evidence.
2. Split the component module into trait/event surface, dialog registrations and
   domain-grouped action definitions while preserving existing public imports and
   variant behavior. Do not replace typed actions with Custom strings.
3. Extract the mux event helper bodies into small focused modules. Coordinate
   helper placement without editing BUG-283-owned navigator files.
4. Split transport implementation bodies into domain delegates and small module
   entrypoints while preserving `Arc<dyn FspecBackend>`, async/object safety,
   default methods, subscription behavior and public construction APIs.
5. Split RPC service state and domain handler bodies. Before changing the
   generated service declaration, validate an approach compatible with the
   existing tarpc service macro: ordinary Rust trait/impl splitting cannot be
   assumed to compose generated RPC declarations. Preserve method/wire names,
   signatures and transport behavior; avoid a protocol redesign.
6. Prove behavior preservation through scoped embedded/WS parity, source-shape,
   board/mux, session-entry and modal regressions. Require all touched production
   files to satisfy the 300-line standard before BOARD-024 integration resumes.

This is a substantial cross-cutting prerequisite, not a small chooser change.
It should not be performed implicitly under the original feature authorization.
If this scope is undesirable, the alternative requires an explicit exception
allowing tiny registrations/delegations in existing oversized files while keeping
all new implementation files below 300 lines. That exception has not been given.

## Shared-file coordination request

BOARD-024 needs exactly one capability-chain registration in
`rust/fspec-tui/src/app/dispatch_capability.rs`:
`|| self.try_dispatch_card_dialog(action)`.
No function reorganization or BUG-283 behavior changes are proposed. The file
remains untouched pending explicit coordination clearance. No BUG-283-owned
navigator files, feature files, tests or documentation may be edited.

## Read-only design findings

Canonical `WorkUnit` preserves unrecognized fields in an `extra` map. The existing
show-work-unit projection omits history and unknown fields; WorkUnitInfo and the
bounded detail API are summaries and cannot satisfy full-card acceptance.

Proposed API: `get_work_unit_details(id: String) -> Result<String, String>`,
returning the complete stored record serialized as JSON. JSON must travel as a
string, not serde_json::Value, because the WebSocket bincode decoder does not
support Value's deserialize_any. Resolve the store from the current service
watcher's workspace, not client-supplied paths or process CWD. Strictly surface
read, parse, malformed-record and missing-target errors; never initialize or
rewrite the store during a detail read.

The TUI can parse locally, render readable sections plus all additional fields,
and follow linked cards through the same API. Keep attachment paths exact.
No security, credentials, daemon restart, installation, commit or push changes
are required or authorized.
