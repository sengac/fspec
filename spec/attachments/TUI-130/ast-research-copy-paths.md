# TUI-130 AST research: copy paths and wrap joins (2026-11-07)

## Pattern `Action::CopyToClipboard($T)`
```
rust/fspec-tui/src/views/provider_settings/copy.rs:108:42:Action::CopyToClipboard(text)
rust/fspec-tui/src/views/provider_settings/oauth_login.rs:149:42:Action::CopyToClipboard(text)
rust/fspec-tui/src/views/agent/turn_modal_select.rs:184:23:Action::CopyToClipboard(text)
rust/fspec-tui/src/views/agent/mouse_dispatch.rs:279:23:Action::CopyToClipboard(text)

```

## Pattern `$C.copy(&$T)`
```
rust/fspec-tui/src/app/dispatch_scroll.rs:225:29:self.clipboard.copy(&text)
rust/fspec-tui/src/app/dispatch_scroll.rs:238:22:self.clipboard.copy(&text)
rust/fspec-tui/src/views/board/details_select.rs:124:29:self.clipboard.borrow_mut().copy(&text)

```

## Pattern `rows.join("\n")`
```
rust/fspec-tui/src/views/agent/turn_modal_select.rs:210:9:rows.join("\n")
rust/fspec-tui/src/views/agent/scrollback_copy.rs:43:9:rows.join("\n")

```

## Pattern `RenderedChunk { $$$ }`
```
rust/fspec-tui/src/views/agent/scrollback_trim.rs:157:17:RenderedChunk {
rust/fspec-tui/src/views/agent/scrollback_copy.rs:158:9:RenderedChunk {
rust/fspec-tui/src/views/agent/scrollback_trim_tests.rs:27:5:RenderedChunk {
rust/fspec-tui/src/views/agent/reflow_cache_tests.rs:36:5:RenderedChunk {
rust/fspec-tui/src/views/agent/scrollback_tests.rs:11:5:RenderedChunk {
rust/fspec-tui/src/store/agent_view/scrollback_mutate.rs:120:25:RenderedChunk {
rust/fspec-tui/src/store/agent_view/scrollback_mutate.rs:145:9:RenderedChunk {

```

Findings: copies reach the OSC 52 writer by three paths (Action::CopyToClipboard, App selection commit calling self.clipboard.copy directly, BoardView's own Osc52Clipboard); scrollback and turn-modal reconstructions join every visual row with \n; RenderedChunk is built in rendered_chunk.rs, scrollback_mutate.rs and scrollback_trim.rs.
