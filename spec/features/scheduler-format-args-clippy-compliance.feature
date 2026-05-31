@done
@code-quality
@scheduler
@RPC-075
Feature: skeleton_invariants clippy fails on uninlined_format_args in codelet-core scheduler

  """
  Workspace-wide clippy lint `clippy::uninlined_format_args` stays at deny (no lint-config relaxation in Cargo.toml)
  Out of scope: any other clippy violations in codelet-core (e.g. the current `NotificationSeverity` unused_imports in session_manager_handle.rs, which is uncommitted WIP from a sibling card and must be cleaned up separately)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All format!/anyhow! invocations in codelet-core/src/scheduler/{agent_job,shell_job}.rs MUST use inline-capture form ({name} / {timestamp} / {e}), never positional {}, var form
  #   2. `cargo clippy -p codelet-core --all-targets -- -D warnings` MUST report zero uninlined_format_args errors after the fix
  #   3. Output strings from format!/anyhow! MUST be byte-identical to the legacy positional form (zero behavioural change)
  #   4. A source-shape regression test MUST assert that no legacy positional format args remain in scheduler/agent_job.rs and scheduler/shell_job.rs
  #
  # EXAMPLES:
  #   1. agent_job.rs:57: format!("[scheduled] {} — {}", name, timestamp) becomes format!("[scheduled] {name} — {timestamp}") and the resulting session_name string is byte-identical
  #   2. shell_job.rs:39-42 multi-line anyhow!("Schedule '{}': shell command is empty", name) collapses to single-line anyhow!("Schedule '{name}': shell command is empty")
  #   3. Running cargo clippy -p codelet-core --all-targets -- -D warnings exits 0 with no uninlined_format_args diagnostics
  #   4. A new test rpc075_scheduler_format_args_shape.rs scans both scheduler files and panics if any format!/anyhow! line still contains a positional `{}` placeholder
  #
  # ========================================

  Background: User Story
    As a developer maintaining the Rust port
    I want to have the scheduler module in codelet-core comply with the workspace-wide `clippy::uninlined_format_args = deny` policy
    So that `scenario_workspace_lints_are_inherited_and_clippy_passes` goes green and stops blocking every subsequent RPC card

  @rust @scheduler @source-shape @lift @rpc-058 @bug-fix @regression
  Scenario: scheduler/agent_job.rs uses inline-capture format args exclusively
    Given the file `codelet/core/src/scheduler/agent_job.rs` exists in the workspace
    When I scan every `format!(` and `anyhow!(` invocation in that file
    Then no invocation contains a positional `{}` placeholder followed by a comma-separated argument list
    And every formatted variable appears inline inside the format string (e.g. `{name}`, `{timestamp}`, `{e}`)

  @rust @scheduler @source-shape @lift @rpc-058 @bug-fix @regression
  Scenario: scheduler/shell_job.rs uses inline-capture format args exclusively
    Given the file `codelet/core/src/scheduler/shell_job.rs` exists in the workspace
    When I scan every `format!(` and `anyhow!(` invocation in that file
    Then no invocation contains a positional `{}` placeholder followed by a comma-separated argument list
    And every formatted variable appears inline inside the format string (e.g. `{name}`, `{command}`, `{e}`)

  @rust @scheduler @lift @rpc-058 @bug-fix @regression @integration-test
  Scenario: cargo clippy on codelet-core passes with -D warnings for the scheduler module
    Given the workspace lint set denies `clippy::uninlined_format_args`
    And the scheduler module uses inline-capture format args
    When I run `cargo clippy -p codelet-core --all-targets -- -D warnings`
    Then no `uninlined_format_args` diagnostic is emitted against `core/src/scheduler/agent_job.rs`
    And no `uninlined_format_args` diagnostic is emitted against `core/src/scheduler/shell_job.rs`

  @rust @scheduler @parity @regression
  Scenario: format! output strings are byte-identical to the legacy positional form
    Given a schedule name of "nightly" and a timestamp of "2026-05-28T00:00:00Z"
    When the agent_job inline-capture form `format!("[scheduled] {name} — {timestamp}")` is evaluated
    Then the resulting string equals `[scheduled] nightly — 2026-05-28T00:00:00Z`
    And the string is byte-identical to the legacy positional form `format!("[scheduled] {} — {}", "nightly", "2026-05-28T00:00:00Z")`
