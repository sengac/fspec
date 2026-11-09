# BUG-349 AST research

## fspec astgrep --pattern 'tokio::time::timeout($$$A)' --lang rust --path rust/fspec-tui/tests/bug210_progress
rust/fspec-tui/tests/bug210_progress/support.rs:122:9:tokio::time::timeout(Duration::from_secs(3), async {
rust/fspec-tui/tests/bug210_progress/support.rs:224:9:tokio::time::timeout(Duration::from_secs(3), entered_rx)
