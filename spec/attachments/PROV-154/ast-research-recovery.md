# Codex recovery research

Rust AST query: `tokio::time::timeout($$$ARGS)` in
`rust/providers/src/codex/recovery.rs` matched lines 48, 68 and 92.
Timeouts bound setup and network reads; UI ticks cannot reset network progress.
Cancellation races both request setup and retry backoff. Accepted streaming
responses are never replayed. Authentication refresh belongs to recovery_auth.

Tests: prov154_codex_recovery and prov154_codex_deadline. Initial failing
suite evidence: /tmp/fspec-codex-board-red.log. Current test evidence is recorded
in /tmp/fspec-codex-completion-providers.log.
