# REFAC-038 AST research

## fspec astgrep --pattern 'hiding($$$A)' --lang rust --path rust/fspec-tui/tests/thinking_indicator_animation_parity_rpc093.rs
rust/fspec-tui/tests/thinking_indicator_animation_parity_rpc093.rs:112:26:hiding(captured.chars().count(), 240, None)
rust/fspec-tui/tests/thinking_indicator_animation_parity_rpc093.rs:133:24:hiding(total, t0, None)
rust/fspec-tui/tests/thinking_indicator_animation_parity_rpc093.rs:148:16:hiding(0, t1 - 102, Some(t1))
rust/fspec-tui/tests/thinking_indicator_animation_parity_rpc093.rs:216:17:hiding(13, t3 - 51, None)
rust/fspec-tui/tests/thinking_indicator_animation_parity_rpc093.rs:232:17:hiding(8, 0, None)

Two more call sites (`hiding(5, 0, None)`, `hiding(28, 0, None)`) sit inside `assert!` macro arguments, which ast-grep does not parse. Before the split each site was a 5-6 line `InputTransitionState::Hiding { .. }` literal. The moved helpers had no callers outside this file.
