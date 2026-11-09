# BUG-400 root cause

`rust/fspec-tui/src/views/board/menu_snapshot.rs::chip_inputs` builds each agent chip with

```rust
status: store.session_status_for(id).copied().unwrap_or(SessionStatus::Idle),
```

A session whose status the TUI has not observed yet is painted with the Idle glyph, a guess the user cannot tell from
a real Idle. Fix: carry `Option<SessionStatus>` in `ChipInput` and paint an explicit "unknown" glyph/style.
