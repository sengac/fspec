# AST research
- `fspec astgrep --pattern 'let _ = $C.send($A);' --lang rust --path rust/fspec-tui/src/app`: find every sibling; this card covers `dispatch_reconnect.rs`. List the others and decide during specifying whether they belong here or in their own card.
- `fspec astgrep --pattern 'if let Ok($X) = $B.list_work_units().await { $$$ }' --lang rust --path rust/fspec-tui/src`
