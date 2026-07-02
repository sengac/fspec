@done
@querying
@cli
@RPC-227
Feature: Port export-dependencies command to Rust
  """
  Core: codelet/fspec-core/src/commands/export_dependencies.rs — pub async fn run(args_json:&str, project_root:&Path)->Result<String,FspecCoreError>. Args: { format: String, output: String }. Reads blocks/blockedBy/dependsOn/relatesTo from WorkUnit.extra. mermaid → string builder; else → IndexMap<String, DepEntry> serialized via to_string_pretty (insertion order, NOT BTreeMap).
  CLI bridge: codelet/fspec/src/export_dependencies.rs (CliArgs { format, output }). clap variant Mode::ExportDependencies with two required positionals. Success: println! the returned message; Error: eprintln! ✗ Failed to export dependencies: <msg>, exit 1. Help config codelet/fspec-core/src/help/configs/export_dependencies.rs + fixture export-dependencies.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Reads spec/work-units.json via ensureWorkUnitsFile (auto-create when missing, escalate malformed JSON)
  #   2. format 'mermaid' emits a graph TB diagram; any other format value (including 'json' and 'dot') emits the JSON dependency map
  #   3. Mermaid node lines append :::done for done status and :::blocked for blocked status; node label is title or id when title missing
  #   4. Mermaid edges: blocks use -->|blocks|, dependsOn use -.->|depends on|, relatesTo use <-.->|relates to|; relatesTo dedupes bidirectionally while blocks/dependsOn dedupe on their own edge key
  #   5. Mermaid output ends with a blank line then classDef done fill:#90EE90 and classDef blocked fill:#FFB6C1
  #   6. JSON output maps every work unit (in insertion order) to {blocks, blockedBy, dependsOn, relatesTo} arrays, missing arrays defaulting to empty, 2-space indent
  #   7. Output file is written after creating parent directories recursively; content has no extra trailing newline
  #   8. On success the CLI prints "✓ Dependencies exported to <output>" to stdout and exits 0
  #   9. On failure the CLI prints "✗ Failed to export dependencies: <message>" to stderr and exits 1
  #   10. Both invocation paths (CLI clap subcommand and LLM dispatcher) converge on the same fspec-core run function
  #
  # EXAMPLES:
  #   1. Export mermaid for a store with blocks/dependsOn/relatesTo writes graph TB with node + edge lines and classDef trailer, prints ✓ Dependencies exported to deps.mmd
  #   2. Export json writes a dependency map keyed by work unit id with blocks/blockedBy/dependsOn/relatesTo arrays in insertion order
  #   3. Export dot falls into the JSON branch and writes the same JSON content as the json format
  #   4. Export mermaid marks a done work unit node with :::done and a blocked one with :::blocked
  #   5. Export mermaid where A relatesTo B and B relatesTo A emits only one <-.->|relates to| edge (bidirectional dedupe)
  #   6. Export to a nested path like out/graphs/deps.json creates the parent directories before writing
  #   7. Export with malformed spec/work-units.json escalates a parse error (Failed to parse work-units.json)
  #   8. Dispatcher and CLI produce identical written file content for the same store and format
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to run export-dependencies in the Rust binary and via the LLM dispatcher
    So that the Rust port emits byte-identical mermaid and JSON dependency graphs and messages as the TypeScript command

  Scenario: CLI export-dependencies mermaid writes the file and prints the success message
    Given a workspace whose spec/work-units.json contains work units with dependencies
    When I run `fspec export-dependencies mermaid deps.mmd`
    Then the command exits with code 0
    And stdout contains "✓ Dependencies exported to deps.mmd"
    And deps.mmd contains a graph TB diagram

  Scenario: CLI export-dependencies json writes the dependency map
    Given a workspace whose spec/work-units.json contains work units with dependencies
    When I run `fspec export-dependencies json deps.json`
    Then the command exits with code 0
    And deps.json contains the dependency map keyed by work unit id

  Scenario: CLI export-dependencies requires the output argument
    Given an empty workspace
    When I run `fspec export-dependencies mermaid`
    Then the command exits with a non-zero code
    And stderr reports a missing required argument

  Scenario: CLI export-dependencies --help prints the help fixture
    Given an empty workspace
    When I run `fspec export-dependencies --help`
    Then stdout matches the captured export-dependencies help fixture

  Scenario: CLI delegates to the same fspec-core function as the dispatcher
    Given a workspace whose spec/work-units.json contains work units with dependencies
    When I export the dependencies via the CLI and via the dispatcher into separate files
    Then both files have identical content
