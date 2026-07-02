@done
@RPC-178
@rust
@cli
@mutation
Feature: Port add-diagram command to Rust
  """
  Framing A divergence — Mermaid validation: The Rust port skips JSDOM + mermaid.render
  validation (the TS path requires a full DOM and the mermaid renderer, neither of which
  exist in Rust without shelling out). Instead it replays only the pure-regex pre-checks
  from src/utils/mermaid-validation.ts:25-48: (1) reject any `subgraph "Quoted Title"` and
  (2) reject subgraph identifiers containing characters outside [A-Za-z0-9_-]. All other
  diagram code strings are accepted as syntactically valid.

  Framing A divergence — FOUNDATION.md regeneration: generate-foundation-md (RPC-233) is
  itself unported. The Rust port writes spec/foundation.json and prints the two CLI status
  lines ('  Updated: spec/foundation.json' + '  Regenerated: spec/FOUNDATION.md') but does
  NOT actually regenerate the markdown file. A follow-up RPC will wire generate-foundation-md
  in once RPC-233 lands.

  Core impl at codelet/fspec-core/src/commands/add_diagram.rs uses
  crate::io::ensure::ensure_foundation_file to load (or auto-create) spec/foundation.json
  (canonical generic schema v2.0.0). It guarantees architectureDiagrams is an array, finds
  any existing diagram with the same title, and either replaces it in place or appends a
  new {title, mermaidCode, description?} object. Persistence uses
  crate::io::locked_file::write_json_atomic so other top-level fields round-trip losslessly.

  Args shape (camelCase JSON): { section: String, title: String, code: String,
  description?: String }. The `section` field is accepted for CLI shape parity but is NOT
  persisted (the generic schema v2.0.0 has no `section` field on architectureDiagrams items).

  Two-front-doors: clap CLI and LLM dispatcher both call commands::add_diagram::run.
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want a Rust implementation of the add-diagram command that matches the TypeScript behaviour
    So that the standalone fspec Rust binary can add Mermaid diagrams to foundation.json without depending on Node.js

  Scenario: Dispatcher appends a new diagram when architectureDiagrams is empty
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram with section='Architecture' title='Component Diagram' code='graph TD\n  A-->B'
    Then the dispatcher returns success=true
    And spec/foundation.json architectureDiagrams contains exactly one entry
    And that entry has title='Component Diagram' and mermaidCode='graph TD\n  A-->B'
    And that entry has no 'section' field

  Scenario: Dispatcher replaces an existing diagram with the same title in place
    Given spec/foundation.json contains a diagram titled 'System Overview' with mermaidCode='graph LR\n  Old-->Content'
    When I dispatch add-diagram with section='Architecture' title='System Overview' code='graph LR\n  New-->Diagram'
    Then the dispatcher returns success=true
    And spec/foundation.json architectureDiagrams length is unchanged
    And the diagram titled 'System Overview' now has mermaidCode='graph LR\n  New-->Diagram'

  Scenario: Dispatcher appends a new diagram alongside an existing one
    Given spec/foundation.json contains a diagram titled 'Diagram 1' with mermaidCode='graph TD\n  A-->B'
    When I dispatch add-diagram with section='Architecture' title='Diagram 2' code='graph TD\n  C-->D'
    Then the dispatcher returns success=true
    And spec/foundation.json architectureDiagrams has length 2
    And the order is ['Diagram 1', 'Diagram 2']

  Scenario: Dispatcher auto-creates spec/foundation.json when missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-diagram with section='Architecture' title='Initial' code='graph TD\n  Start-->End'
    Then the file spec/foundation.json exists
    And the dispatcher returns success=true
    And spec/foundation.json architectureDiagrams contains exactly one entry titled 'Initial'

  Scenario: Dispatcher persists optional description when provided
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram with section='Architecture' title='With Notes' code='graph TD\n  A-->B' description='A flowchart'
    Then the dispatcher returns success=true
    And the diagram titled 'With Notes' has description='A flowchart'

  Scenario: Dispatcher rejects empty section argument
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram with section='' title='X' code='graph TD\n  A-->B'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Section name cannot be empty'

  Scenario: Dispatcher rejects empty title argument
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram with section='Architecture' title='' code='graph TD\n  A-->B'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Diagram title cannot be empty'

  Scenario: Dispatcher rejects empty code argument
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram with section='Architecture' title='X' code=''
    Then the dispatcher returns success=false
    And the error message contains the substring 'Diagram code cannot be empty'

  Scenario: Dispatcher rejects diagram with quoted subgraph title (merman pre-check)
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram with section='Architecture' title='Bad' code='graph TB\n  subgraph "Quoted"\n  end'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Quoted subgraph titles are not supported'

  Scenario: Dispatcher rejects diagram with invalid subgraph identifier (merman pre-check)
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram with section='Architecture' title='Bad' code='graph TB\n  subgraph Id!!!\n  end'
    Then the dispatcher returns success=false
    And the error message contains the substring "Invalid subgraph identifier 'Id!!!'"

  Scenario: Dispatcher preserves unknown top-level fields on write
    Given spec/foundation.json contains a top-level 'project' object and a custom 'experiments' key
    When I dispatch add-diagram with section='Architecture' title='X' code='graph TD\n  A-->B'
    Then the dispatcher returns success=true
    And spec/foundation.json still contains the 'experiments' key with its original value
    And spec/foundation.json still contains the 'project' object

  Scenario: Dispatcher rejects diagram with a genuine merman syntax error
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram with section='Architecture' title='Bad' code='flowchart TD\n  A[Start --> B[Done'
    Then the dispatcher returns success=false
    And no entry is appended to architectureDiagrams

  Scenario: Dispatcher rejects code that matches no mermaid diagram type
    Given spec/foundation.json exists with architectureDiagrams=[]
    When I dispatch add-diagram with section='Architecture' title='Bad' code='this is not a diagram at all'
    Then the dispatcher returns success=false
    And no entry is appended to architectureDiagrams
