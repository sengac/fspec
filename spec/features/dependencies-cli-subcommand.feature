@done
@querying
@cli
@RPC-224
Feature: dependencies clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `dependencies` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern (architecture note on RPC-253, reused for RPC-224):
  - Shell argv         → clap → codelet/fspec/src/dependencies.rs → fspec_core::commands::dependencies::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::dependencies::run
  Both call sites pass a JSON-encoded args shape ({workUnitId, graph}) and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS process.cwd() default).
  The clap subcommand exposes one required positional <work-unit-id> and a boolean --graph flag (default false, long only — no short).
  Default (non-graph) output is the rendered 'Dependencies for <id>:' block; --graph renders the depth-first blocks tree.
  Exit-code contract: 0 on success, 1 on any FspecCoreError. Missing work units surface an AI-friendly system-reminder on stderr.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/dependencies.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a dependencies clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that dependency-display logic is never duplicated and byte-parity with the TS CLI is preserved

  Scenario: Clap exposes dependencies as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec dependencies --help` from a shell
    Then the command exits 0
    Then stdout contains the substring 'dependencies'

  Scenario: CLI prints the header and relationship lines for a unit with dependencies
    Given a project root whose spec/work-units.json contains AUTH-001 with blocks=['AUTH-002'] and dependsOn=['SCHEMA-001']
    When I run `./codelet/target/release/fspec dependencies AUTH-001` from that workspace
    Then the command exits 0
    Then stdout contains the substring 'Dependencies for AUTH-001:'
    Then stdout contains the substring 'Blocks: AUTH-002'
    Then stdout contains the substring 'Depends on: SCHEMA-001'

  Scenario: CLI --graph prints the depth-first blocks tree
    Given a project root whose spec/work-units.json contains AUTH-001 with blocks=['AUTH-002'] and AUTH-002 with no relationships
    When I run `./codelet/target/release/fspec dependencies AUTH-001 --graph` from that workspace
    Then the command exits 0
    Then stdout contains the substring 'AUTH-001'
    Then stdout contains the substring 'blocks → AUTH-002'

  Scenario: CLI exits 1 and writes to stderr when the work unit does not exist
    Given a project root whose spec/work-units.json contains AUTH-001 only
    When I run `./codelet/target/release/fspec dependencies INVALID-999` from that workspace
    Then the command exits with code 1
    Then stderr contains the substring 'does not exist'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/work-units.json contains AUTH-001 with blocks=['AUTH-002'] and AUTH-002 with no relationships
    When I dispatch dependencies through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    Then the DispatchResult.data equals the stdout produced by running `./codelet/target/release/fspec dependencies AUTH-001`
    Then the CLI bridge module codelet/fspec/src/dependencies.rs contains NO inline rendering, traversal, or filter logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: dependencies --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec dependencies --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/dependencies.txt
    Then stdout starts with a blank line followed by 'DEPENDENCIES'
