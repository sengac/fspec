@done
@RPC-233
@rust
@cli
@mutation
Feature: Port generate-foundation-md command to Rust

  """
  Rust port of src/commands/generate-foundation-md.ts (RPC-233), child of
  RPC-003 / epic 'rust-cli-port'. This port also REVERSES the "Framing A"
  divergence noted across the foundation-mutation cards: previously the Rust
  core never touched spec/FOUNDATION.md (generate-foundation-md was a stub).
  With this port, the markdown IS regenerated, and the six Event-Storm
  foundation commands wire crate::commands::generate_foundation_md::regenerate
  in AFTER their atomic write.

  Core impl at codelet/fspec-core/src/commands/generate_foundation_md.rs reads
  spec/foundation.json (existsSync-equivalent check, no auto-create), runs a
  lightweight Mermaid pre-check over architectureDiagrams, renders markdown via
  crate::generators::generate_foundation_md, and writes spec/FOUNDATION.md (or a
  custom --output path) with NO trailing newline — matching the TS
  writeFile(..., markdown, 'utf-8') call byte-for-byte.

  Framing A divergences (documented, not bugs):
  * JSON schema validation: TS calls validateFoundationJson (Ajv) and aborts on
    failure. No Ajv equivalent is ported (validate-foundation-schema still
    NotYetPorted), so this port SKIPS schema validation. Valid foundations
    produce byte-identical output; the Ajv error text cannot be reproduced.
  * Mermaid validation: TS uses mermaid.parse() + jsdom. Rust uses the
    pure-string pre-check from add_diagram.rs (quoted-subgraph-title +
    invalid-identifier detection only). Generated diagrams always pass, so valid
    foundations render identically.

  Args shape (camelCase JSON): { output?: String }. Relative output paths are
  joined to the project root, mirroring TS join(cwd, outputPath).

  Two front doors: clap CLI and LLM dispatcher both call
  commands::generate_foundation_md::run.

  Verified byte-for-byte against the TS implementation during the parity sweep:
  generate-foundation-md → FOUNDATION.md identical (30592 bytes), and all six
  regenerating foundation commands produce byte-identical FOUNDATION.md.
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want a Rust implementation of generate-foundation-md that matches the TypeScript surface
    So that the standalone fspec Rust binary can regenerate FOUNDATION.md from foundation.json without depending on Node.js

  Scenario: Dispatcher generates FOUNDATION.md from foundation.json
    Given spec/foundation.json exists with a valid generic foundation
    When I dispatch generate-foundation-md with no output override
    Then generation succeeds
    And the bytes written to spec/FOUNDATION.md exactly equal the rendered markdown
    And the response message is "Generated spec/FOUNDATION.md from spec/foundation.json"

  Scenario: Dispatcher writes to a custom --output path
    Given spec/foundation.json exists with a valid generic foundation
    When I dispatch generate-foundation-md with output='docs/FOUNDATION.md'
    Then generation succeeds
    And the file docs/FOUNDATION.md is written relative to the project root
    And the response message is "Generated docs/FOUNDATION.md from spec/foundation.json"

  Scenario: Dispatcher fails when foundation.json is missing
    Given an empty project root directory with no spec/foundation.json
    When I dispatch generate-foundation-md with no output override
    Then generation fails
    And the error message contains the substring 'foundation.json not found: spec/foundation.json'
    And the file spec/FOUNDATION.md is not created

  Scenario: Renders header-only output when no optional sections are present
    Given a foundation with a project name but no problem space, solution space, personas, or diagrams
    When the markdown is generated
    Then only the title header is rendered

  Scenario: Falls back to a default header when no project name is set
    Given a foundation with no project name
    When the markdown is generated
    Then a fallback header is rendered

  Scenario: Reports a single invalid diagram with a singular failure message
    Given architectureDiagrams contains one diagram that fails the Mermaid pre-check
    When generation runs
    Then the error message starts with "Diagram validation failed!\n\nFound 1 invalid diagram:\n"
    And the error message ends with "Run 'fspec generate-foundation-md' again after fixing"

  Scenario: Reports multiple invalid diagrams with a pluralised failure message
    Given architectureDiagrams contains two diagrams that fail the Mermaid pre-check
    When generation runs
    Then the error message contains "Found 2 invalid diagrams:"

  Scenario: regenerate is best-effort and swallows failures for mutation callers
    Given a project root with no spec/foundation.json
    When crate::commands::generate_foundation_md::regenerate is invoked
    Then no panic occurs and no FOUNDATION.md is written

  Scenario: Reports a diagram with a genuine merman syntax error as invalid
    Given spec/foundation.json has one architectureDiagram whose mermaidCode has an unterminated node label
    When I dispatch generate-foundation-md
    Then generation fails reporting 'Found 1 invalid diagram:'


  Scenario: All generator-emitted diagrams parse cleanly under merman
    Given a foundation with bounded contexts, aggregates, commands and events
    When the FOUNDATION.md generator emits the bounded-context map and per-context event-flow diagrams
    Then every emitted diagram validates as Ok under merman so valid foundations still render byte-identically

