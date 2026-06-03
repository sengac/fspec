@done
@validation
@regression
@source-shape
@code-quality
@agent-core
@rust
@RPC-077
Feature: skeleton_invariants clippy fails on redundant_clone in codelet-sessions handle_impl

  """
  The `redundant_clone` violations were resolved implicitly during the RPC-082/083/084/086 workspace clippy sweep (turns 4966-5169 of the prior session) when handle_impl.rs was restructured. Both offending clones at the original lines 1321 (`id: id.clone(),`) and 1323 (`prompt: prompt.clone(),`) no longer exist in the source. RPC-077 lands the regression coverage via a structural source-string assertion test + verification that `scenario_workspace_lints_are_inherited_and_clippy_passes` is green.
  The pre-existing test `scenario_workspace_lints_are_inherited_and_clippy_passes` in `codelet/sessions/tests/skeleton_invariants.rs:336-372` already pins the runtime contract (clippy exit code 0). RPC-077 adds a fast structural source-string regression test in `codelet/sessions/tests/rpc077_handle_impl_redundant_clone_shape.rs` that fails in milliseconds if the offending `id: id.clone(),` or `prompt: prompt.clone(),` patterns ever return — without paying the 30-second clippy compile cost on every CI run.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. codelet/sessions/src/handle_impl.rs must NOT contain `id: id.clone(),` or `prompt: prompt.clone(),` inside a struct literal whose owned bindings (`id`, `prompt`) are dropped at the end of the block
  #   2. `cargo clippy -p codelet-sessions --all-targets -- -D warnings` must complete with exit code 0
  #   3. `cargo test -p codelet-sessions --test skeleton_invariants -- scenario_workspace_lints_are_inherited_and_clippy_passes` must pass
  #
  # EXAMPLES:
  #   1. Searching codelet/sessions/src/handle_impl.rs for the regex `id: id\.clone\(\),` returns zero matches
  #   2. Searching codelet/sessions/src/handle_impl.rs for the regex `prompt: prompt\.clone\(\),` returns zero matches
  #   3. Running `cargo clippy -p codelet-sessions --all-targets -- -D warnings` from the codelet workspace exits with status 0 and emits no `redundant_clone` warning
  #   4. Running `cargo test -p codelet-sessions --test skeleton_invariants -- scenario_workspace_lints_are_inherited_and_clippy_passes` from the codelet workspace prints `test result: ok. 1 passed`
  #
  # ========================================

  Background: User Story
    As a fspec developer
    I want to see `cargo clippy -p codelet-sessions --all-targets -- -D warnings` complete without errors
    So that the skeleton_invariants regression test stays green and codelet-sessions does not regress on `clippy::redundant_clone`

  Scenario: handle_impl.rs has no redundant `id: id.clone(),` struct literal
    Given the source of `codelet/sessions/src/handle_impl.rs`
    When I scan the source for the substring `id: id.clone(),`
    Then zero matches are found


  Scenario: handle_impl.rs has no redundant `prompt: prompt.clone(),` struct literal
    Given the source of `codelet/sessions/src/handle_impl.rs`
    When I scan the source for the substring `prompt: prompt.clone(),`
    Then zero matches are found


  Scenario: codelet/sessions/Cargo.toml inherits workspace lints
    Given the source of `codelet/sessions/Cargo.toml`
    When I inspect the `[lints]` section
    Then the section declares `workspace = true`

