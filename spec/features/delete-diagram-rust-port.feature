@done
@RPC-216
@rust
@cli
@mutation
Feature: Port delete-diagram command to Rust
  """
  Framing A divergence — match-by-title: The TS implementation filters
  architectureDiagrams with `d.section === section && d.title === title`, using a
  legacy DiagramSection.section field that add-diagram NEVER writes (the generic schema
  v2.0.0 has no `section` field). In practice the TS command can never find any diagram
  written by add-diagram — a latent TS bug. The Rust port matches by `title` ONLY. The
  `section` argument is accepted for CLI shape parity and echoed verbatim in success
  and error messages, but does NOT participate in lookup.

  Framing A divergence — FOUNDATION.md regeneration: generate-foundation-md (RPC-233)
  is unported. The Rust port writes spec/foundation.json and prints the two CLI status
  lines ('  Updated: spec/foundation.json' + '  Regenerated: spec/FOUNDATION.md') but
  does NOT actually regenerate the markdown file.

  No auto-create: Unlike add-diagram, the TS delete-diagram uses `existsSync` directly
  and errors when spec/foundation.json is missing. The Rust port preserves this — it
  uses std::fs::read_to_string + an explicit ENOENT check and does NOT route through
  ensure_foundation_file.

  Core impl at codelet/fspec-core/src/commands/delete_diagram.rs reads
  spec/foundation.json (no auto-create), finds the first diagram with matching title,
  removes it via Vec::remove, and persists via crate::io::locked_file::write_json_atomic
  so other top-level fields round-trip losslessly.

  Args shape (camelCase JSON): { section: String, title: String }. Both required.

  Two-front-doors: clap CLI and LLM dispatcher both call commands::delete_diagram::run.
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want a Rust implementation of the delete-diagram command that matches the TypeScript surface
    So that the standalone fspec Rust binary can remove Mermaid diagrams from foundation.json without depending on Node.js

  Scenario: Dispatcher removes an existing diagram by title
    Given spec/foundation.json contains a diagram titled 'Component Flow' with mermaidCode='graph TD\n  A-->B'
    When I dispatch delete-diagram with section='Architecture' title='Component Flow'
    Then the dispatcher returns success=true
    And spec/foundation.json architectureDiagrams is empty
    And the response message contains the substring "Deleted diagram 'Component Flow' from section 'Architecture'"

  Scenario: Dispatcher fails when spec/foundation.json is missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch delete-diagram with section='Architecture' title='Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring 'foundation.json not found: spec/foundation.json'
    And the file spec/foundation.json still does NOT exist

  Scenario: Dispatcher fails when title is not found
    Given spec/foundation.json contains a diagram titled 'Existing' with mermaidCode='graph TD\n  A-->B'
    When I dispatch delete-diagram with section='Architecture' title='Missing'
    Then the dispatcher returns success=false
    And the error message contains the substring "Diagram 'Missing' not found in section 'Architecture'"
    And spec/foundation.json architectureDiagrams still contains the diagram titled 'Existing'

  Scenario: Dispatcher removes only the middle entry from a list of three diagrams
    Given spec/foundation.json contains diagrams titled 'First', 'Middle', 'Last' in that order
    When I dispatch delete-diagram with section='Architecture' title='Middle'
    Then the dispatcher returns success=true
    And spec/foundation.json architectureDiagrams has length 2
    And the remaining diagrams are ['First', 'Last'] in that order

  Scenario: Dispatcher leaves architectureDiagrams as an empty array after removing the only entry
    Given spec/foundation.json contains exactly one diagram titled 'OnlyOne'
    When I dispatch delete-diagram with section='Architecture' title='OnlyOne'
    Then the dispatcher returns success=true
    And spec/foundation.json architectureDiagrams equals []
    And spec/foundation.json still has its 'project' object intact

  Scenario: Dispatcher preserves unknown top-level fields on write
    Given spec/foundation.json contains a diagram titled 'X' and a top-level 'experiments' key
    When I dispatch delete-diagram with section='Architecture' title='X'
    Then the dispatcher returns success=true
    And spec/foundation.json still contains the 'experiments' key with its original value

  Scenario: Dispatcher matches by title only — section argument is informational (Framing A)
    Given spec/foundation.json contains a diagram titled 'OneTitle' (no 'section' field on the entry)
    When I dispatch delete-diagram with section='AnythingElse' title='OneTitle'
    Then the dispatcher returns success=true
    And spec/foundation.json architectureDiagrams is empty
    And the response message echoes the supplied section 'AnythingElse'
