# BOARD-024: board Enter chooser and complete card details

## Authorization and scope

Implement `/tmp/fspec-board-enter-task.md` in the existing native session.
Do not install, restart, commit, push, or change security configuration.
Preserve BUG-280/281/282. BUG-283 owns navigator.rs, navigator_render.rs,
navigator_events.rs and its feature, tests and attachments; none may be edited.
Report any required dispatch_capability.rs overlap before editing.

## Requirements and acceptance

- Board and mux card Enter opens a modal with Open in agent selected by default
  and Show full details as the other choice. Capture original card and origin.
- Agent choice delegates to existing EnterWorkUnit/MuxEnterWorkUnit behavior,
  preserving earliest associated session, normal/isolated creation, ambient
  session associations and mux layout behavior.
- Details load canonical stored information rather than truncated board summaries:
  full description, metadata, links/dependencies, example mapping, timestamps,
  history, attachments and all applicable stored fields.
- Wrap Unicode text; arrows, PageUp/PageDown, Home/End and mouse wheel scroll.
  Resize preserves focus and selection; narrow terminals remain usable.
- Tab/Shift-Tab selects linked cards and attachments. Enter follows a linked card
  in the same details flow or uses existing exact-path attachment viewing.
- Escape unwinds linked-card history, details, chooser, then returns to the
  original board or mux without changing session or selection. Modal keys do not
  leak. Missing cards, missing attachments, read errors and loading are visible
  and retain context. Footer/help documents the interaction.

## Intended design

Use compositor dialogs and a new board-card chooser action emitted by board.rs
and navigator_mux_events.rs. Keep dispatch_work_unit_entry.rs unchanged.
Read full details through a dedicated transport-independent backend method shared
by embedded and WebSocket transports; do not bloat WorkUnitInfo literals.
Separate dialog interaction, canonical projection and rendering into small files.

## ACDD and validation plan

Feature first, 1:1 scenario integration tests second, causal behavioral RED before
production changes. Exercise actual Board/Mux key dispatch through App and rendered
buffers, Unicode/narrow/resize, linked history, exact attachment paths and errors.
Run scoped ci-test/debug0 Cargo commands with low job counts, relevant entry,
attachment, modal and mux regressions, compilation, Clippy and formatting checks.
Record exact commands and raw RED/GREEN/quality receipts in evidence attachments.
Perform independent review and verify canonical Done plus registered attachments;
passing tests alone is not completion.
