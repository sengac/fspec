@done
@querying
@cli
@RPC-227
Feature: Port export-dependencies command to Rust
  """
  Core: rust/fspec-core/src/commands/export_dependencies.rs — pub async fn run(args_json:&str, project_root:&Path)->Result<String,FspecCoreError>. Args: { format: String, output: String }. Reads blocks/blockedBy/dependsOn/relatesTo from WorkUnit.extra. mermaid → string builder; else → IndexMap<String, DepEntry> serialized via to_string_pretty (insertion order, NOT BTreeMap).
  CLI bridge: rust/fspec/src/export_dependencies.rs (CliArgs { format, output }). clap variant Mode::ExportDependencies with two required positionals. Success: println! the returned message; Error: eprintln! ✗ Failed to export dependencies: <msg>, exit 1. Help config rust/fspec-core/src/help/configs/export_dependencies.rs + fixture export-dependencies.txt.
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

  Scenario: Export mermaid for a store with all dependency kinds
    Given a work units store where AUTH-001 blocks AUTH-002, depends on AUTH-003, and relates to AUTH-004
    When I export dependencies in mermaid format to deps.mmd
    Then the written content starts with "graph TB"
    And it contains a node line for each work unit
    And it contains the edge lines for blocks, depends on, and relates to
    And it ends with the classDef done and classDef blocked trailer
    And the returned message is "✓ Dependencies exported to deps.mmd"

  Scenario: Export json for a store maps every work unit to dependency arrays
    Given a work units store where AUTH-001 blocks AUTH-002 and depends on AUTH-003
    When I export dependencies in json format to deps.json
    Then the written JSON keys every work unit in insertion order
    And each entry has blocks, blockedBy, dependsOn, and relatesTo arrays

  Scenario: Export dot format produces the same JSON content as json format
    Given a work units store containing AUTH-001
    When I export dependencies in dot format to deps.dot
    Then the written content equals the json format output for the same store

  Scenario: Export mermaid marks done and blocked nodes
    Given a work units store where AUTH-002 is done and AUTH-005 is blocked
    When I export dependencies in mermaid format to deps.mmd
    Then the AUTH-002 node line ends with ":::done"
    And the AUTH-005 node line ends with ":::blocked"

  Scenario: Export mermaid dedupes bidirectional relatesTo edges
    Given a work units store where AUTH-001 relates to AUTH-004 and AUTH-004 relates to AUTH-001
    When I export dependencies in mermaid format to deps.mmd
    Then only one "<-.->|relates to|" edge appears between AUTH-001 and AUTH-004

  Scenario: Export to a nested output path creates parent directories
    Given a work units store containing AUTH-001
    When I export dependencies in json format to out/graphs/deps.json
    Then the parent directory out/graphs is created and the file is written

  Scenario: Export with a malformed work units file escalates a parse error
    Given a spec/work-units.json file that is not valid JSON
    When I export dependencies in json format to out.json
    Then the run returns an error containing "Failed to parse work-units.json"

  Scenario: Dispatcher and core produce identical file content
    Given a work units store containing AUTH-001 with dependencies
    When I export dependencies in json format via the core run function
    Then the written file content is the same as exporting via the dispatcher path
