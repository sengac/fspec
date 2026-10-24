# Codex CLI initialization boundary

AstGrep located `install_slash_command` in `rust/fspec-core/src/commands/init.rs:492`.
Codex CLI shares the home prompt installer with Codex; `init::run` preserves
existing project configuration through read-modify-write. Tests exercise actual
files through the dispatcher with isolated HOME and serial process-state access.
AGENT-018 already implemented the behavior; regression tests need no production
changes and must not manufacture a historical red phase.
