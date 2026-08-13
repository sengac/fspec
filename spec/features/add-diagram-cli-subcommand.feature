@done
@RPC-178
@rust
@cli
@mutation
Feature: fspec add-diagram CLI subcommand (Rust port)
  """
  Clap derive subcommand `add-diagram` mirrors the TS Commander.js registration at
  src/commands/add-diagram.ts:162-169 — three required positional arguments
  `<section>`, `<title>`, and `<code>`. The CLI bridge at rust/fspec/src/add_diagram.rs
  marshals the clap args into a JSON object and delegates to
  codelet_fspec_core::commands::add_diagram::run; no validation, mermaid checks, or file
  IO logic is duplicated in the bridge.

  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed
  with 'Error:' (parity with the chalk-red TS error path at src/commands/add-diagram.ts:148-158).

  The `fspec add-diagram --help` output is byte-for-byte identical to
  `node dist/index.js add-diagram --help` piped to non-TTY (captured fixture at
  rust/fspec/tests/fixtures/help/add-diagram.txt).
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want an `add-diagram` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes add-diagram with three positional args in --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec add-diagram --help`
    Then the command exits 0
    And stdout describes the add-diagram subcommand
    And stdout mentions the `<section>` argument
    And stdout mentions the `<title>` argument
    And stdout mentions the `<code>` argument

  Scenario: CLI adds a new diagram and prints the success block on stdout
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I run `./rust/target/release/fspec add-diagram Architecture "Component" "graph TD\n  A-->B"`
    Then the command exits 0
    And stdout contains the line '✓ Added diagram "Component"'
    And stdout contains the substring '  Updated: spec/foundation.json'
    And stdout contains the substring '  Regenerated: spec/FOUNDATION.md'
    And spec/foundation.json architectureDiagrams contains exactly one entry titled 'Component'

  Scenario: CLI replaces an existing diagram and prints the Updated success block
    Given spec/foundation.json contains a diagram titled 'System Overview' with mermaidCode='graph LR\n  Old-->X'
    When I run `./rust/target/release/fspec add-diagram Architecture "System Overview" "graph LR\n  New-->X"`
    Then the command exits 0
    And stdout contains the line '✓ Updated diagram "System Overview"'
    And the diagram titled 'System Overview' now has mermaidCode='graph LR\n  New-->X'

  Scenario: CLI rejects an empty code argument with exit 1 and stderr Error prefix
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I run `./rust/target/release/fspec add-diagram Architecture "X" ""`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Diagram code cannot be empty'

  Scenario: CLI rejects an invalid mermaid subgraph identifier (merman pre-check)
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I run `./rust/target/release/fspec add-diagram Architecture "Bad" "graph TB\n  subgraph Id!!!\n  end"`
    Then the command exits with code 1
    And stderr contains the substring "Invalid subgraph identifier 'Id!!!'"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram via fspec_core::dispatch::dispatch_command with section='Architecture' title='Via dispatcher' code='graph TD\n  A-->B'
    Then the dispatcher writes spec/foundation.json
    And running `./rust/target/release/fspec add-diagram Architecture "Via CLI" "graph TD\n  C-->D"` afterwards exits 0
    And spec/foundation.json architectureDiagrams contains two entries
    And the CLI bridge module rust/fspec/src/add_diagram.rs contains NO inline mermaid validation, ensure_foundation_file, or JSON-mutation logic — its only computation is JSON arg marshalling

  Scenario: add-diagram --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec add-diagram --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/add-diagram.txt
