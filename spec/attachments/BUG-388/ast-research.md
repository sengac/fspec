# BUG-388 AST research

- `grep -n "broadcast::channel(64)" rust/fspec-tui/tests/common/mock_default.rs` → lines 8-16, nine channels.
- `grep -n "Mutex::new(Some(" rust/fspec-tui/tests/common/mock_default.rs` → the nine matching field initialisers.
- `wc -l rust/fspec-tui/tests/common/*.rs` → only `mock_default.rs` reaches 300.
