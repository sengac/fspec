# BUG-424 root cause

client_mcp_approval_regressions defines an unused env macro (compiler warning)

rust/fspec/tests/client_mcp_approval_regressions.rs:5 defines macro_rules! env to redirect CARGO_MANIFEST_DIR for its #[path] fixtures; rustc warns 'unused macro definition: env' on both cargo test and clippy. Either the redirect is dead (remove it) or the fixtures it was meant for now resolve their paths another way. The file also pulls in tests through #[path] fragments.

Found while running the BUG-399 regression targets (2026-11-03); filed to the backlog untouched per the user's instruction.
