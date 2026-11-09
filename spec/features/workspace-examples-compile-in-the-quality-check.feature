@wip
@tui
@code-quality
@bug-fix
@rust
@BUG-632
Feature: Workspace examples compile in the quality check
  """
  TUI-130 added RenderedChunk::row_ends (how each wrapped row ends in copied text). The fspec-tui example scrollback_perf still built RenderedChunk without it, and no suite builds examples (cargo test and clippy --tests skip example targets). The example now takes lines and row ends from one wrap_source_rows call. Each example of fspec-tui and codelet-sessions is compiled as a module into a guard test of its crate (#[path] include), so the crate's suite fails to build when an example does, and the guard lists the crate's examples directory so a new example cannot be left out. spec/fspec-config.json is local and gitignored, so the guard lives in committed test targets; the local quality check also runs clippy --workspace --all-targets.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The fspec-tui example scrollback_perf builds each RenderedChunk with the lines and row ends of one wrap_source_rows call, so it compiles against the TUI-130 RenderedChunk
  #   2. Every example of fspec-tui and of codelet-sessions is compiled into a test target of its crate, so the crate's test suite fails to build when an example does; a test lists the crate's examples directory and fails when an example is not compiled in
  #
  # EXAMPLES:
  #   1. cargo clippy -p codelet-fspec-tui --examples -- -D warnings at HEAD fails with E0063 missing field row_ends in scrollback_perf.rs:50; after the fix it passes
  #   2. A new fspec-tui example added without being compiled into the guard test fails that test, naming the example file
  #   3. sessions/examples/repro_paused_chunk_delay.rs breaks after an API change: cargo test -p codelet-sessions fails to build its guard test
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to have every workspace example compile in the checks the suites run
    So that an API change cannot leave an example broken unnoticed and break clippy --all-targets

  Scenario: The scrollback_perf example builds its chunks with the row ends of their wrap
    Given the fspec-tui example scrollback_perf
    When it pushes a wrapped chunk onto its ScrollbackList
    Then the RenderedChunk takes its lines and its row ends from one wrap_source_rows call

  Scenario: Every fspec-tui example is compiled into the fspec-tui test suite
    Given the examples directory of fspec-tui
    When the fspec-tui guard test target is built
    Then each example file is compiled into it as a module
    And an example file that is not compiled in fails the guard test, naming the file

  Scenario: Every codelet-sessions example is compiled into the codelet-sessions test suite
    Given the examples directory of codelet-sessions
    When the codelet-sessions guard test target is built
    Then each example file is compiled into it as a module
    And an example file that is not compiled in fails the guard test, naming the file
