@done
@CONFIG-009
@cli
@validation
@config-management
Feature: Config Scope Validation
  """
  CONFIG-009 — Config-scope key ownership rule + `fspec validate-config` validation surface.

  Key-scope ownership (codified rule): user-scope ~/.fspec/fspec-config.json holds
  user-owned state (providers, tui.lastUsedModel, tui.mux, tui.defaultThinkingLevel, rlcd,
  optionally research credentials). Project-scope spec/fspec-config.json holds
  repo-owned state (tools, agent, optionally research credentials). A key whose only
  writer targets one scope must never appear in the other scope's file.

  validate-config (Rust-only extension command, DISC-003 pattern like foundation-status):
  - Two front doors converge on a single `pub async fn run(args_json, project_root)` in
  codelet-fspec-core/src/commands/validate_config.rs. Envelope: {valid, exitCode,
  message, warnings[]} — warnings are non-fatal (exitCode 0 when the user file parses;
  only an unreadable/malformed user file yields a reported parse warning, never a crash).
  - Read-only: never writes config.
  - Resolves the user dir: FSPEC_USER_DIR env override first, then HOME/.fspec (testable
  without touching a real ~/.fspec).
  - Flags ONLY project-only top-level keys (tools, agent) present in the USER-scope file,
  with per-key remediation text. `research` is never flagged (legitimate user fallback).
  - Wiring: commands/mod.rs + dispatch.rs run_ported arm + canonical.rs PORTED_COMMANDS
  (+extension arm in dispatch_command) + help/configs/validate_config.rs +
  help_dispatch_table.rs + clap Mode::ValidateConfig in rust/fspec/src/main.rs + thin
  CLI bridge rust/fspec/src/validate_config.rs.
  - Out of scope (deferred): dropping the init.rs user-level tools fallback (TS parity);
  symmetric project-scope validation; automated repair of contaminated configs.
  """

  Background: User Story
    As a fspec user
    I want to know which config-scope keys belong where, with a validation surface that detects contamination
    So that project-scope keys leaking into ~/.fspec/fspec-config.json are detectable instead of silently inert

  Scenario: Project-scope keys in the user config are flagged as warnings
    Given a user-scope config file containing tools, agent, and providers keys
    When I run validate-config
    Then validate-config reports a tools warning with remediation text
    And validate-config reports an agent warning with remediation text

  Scenario: A user-scope-only config passes validation without warnings
    Given a user-scope config file containing only providers, tui, rlcd, and research keys
    When I run validate-config
    Then validate-config reports zero project-scope warnings

  Scenario: A missing user config file validates cleanly
    Given no user-scope config file exists at all
    When I run validate-config
    Then validate-config reports zero project-scope warnings and does not fail

  Scenario: A malformed user config file is reported as a warning, not a crash
    Given a user-scope config file containing invalid JSON
    When I run validate-config
    Then validate-config reports a parse warning and does not crash
