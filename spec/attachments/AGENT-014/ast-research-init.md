# Codex initialization boundary

AstGrep located `install_slash_command` in `rust/fspec-core/src/commands/init.rs:492`.
The function redirects Codex and Codex CLI prompts to injectable HOME. The shared
`init::run` entry point installs documentation, persists selection, and reports
activation instructions for both front doors. AGENT-018 already implemented this
behavior; this card adds integration regression coverage, not a new implementation.
