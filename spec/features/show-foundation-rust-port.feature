@done
@querying
@cli
@RPC-305
Feature: Port show-foundation command to Rust
  """
  Uses ensure_foundation_file from crate::io::ensure (auto-creating spec/foundation.json with canonical v2.0.0 defaults when missing) — TS source-of-truth at src/commands/show-foundation.ts:75 calls ensureFoundationFile, so the Rust port matches that auto-create behaviour exactly.

  When args.draft=true, reads spec/foundation.json.draft via std::fs::read_to_string directly with explicit ErrorKind::NotFound handling. Does NOT auto-create the draft. Missing draft surfaces 'No draft found at spec/foundation.json.draft. Run `fspec discover-foundation` to create one.' in the success/error envelope (NOT a FspecCoreError).

  FIELD_MAP is a compile-time constant slice mirroring src/commands/show-foundation.ts:30-42: projectName→project.name, projectVision→project.vision, projectType→project.projectType, problemTitle→problemSpace.primaryProblem.title, problemDescription→problemSpace.primaryProblem.description, problemImpact→problemSpace.primaryProblem.impact, solutionOverview→solutionSpace.overview, projectOverview→solutionSpace.overview (legacy), problemDefinition→problemSpace.primaryProblem.description (legacy).

  Field path resolution: 1) if section key is in FIELD_MAP, use the mapped dotted path; 2) otherwise treat section as a dotted path; 3) walk the foundation Value with serde_json::Value::get(part) — returns None if any segment is missing.

  Text renderer parity (src/commands/show-foundation.ts:150-213): build the multi-line text block via a Vec<String> push/join('\n') pattern with sections '=== PROJECT ===', '=== PROBLEM SPACE ===', '=== SOLUTION SPACE ===', '=== PERSONAS ===', '=== ARCHITECTURE DIAGRAMS ==='. Each section gates on the relevant foundation field being present (problemSpace.primaryProblem, solutionSpace, personas.length>0, architectureDiagrams.length>0). Within a section, N/A fallbacks for missing strings ('Name: ${... || 'N/A'}', etc.).

  format='json' (the default fallback for anything not equal to 'text') ALWAYS emits JSON.stringify(displayData, null, 2) — 2-space pretty-printed JSON.

  format='text' branching: if section is present, behave on resolved value type — string → emit raw string verbatim; otherwise → emit JSON.stringify(value, null, 2). If section is NOT present, render via the multi-line PROJECT/PROBLEM SPACE/SOLUTION SPACE/PERSONAS/ARCHITECTURE DIAGRAMS block.

  When args.output is set, write the formatted output to that path (UTF-8) via std::fs::write. The dispatcher path returns the same formatted string AND wrote the file; the CLI bridge interprets args.output presence to alter stdout (prints '✓ Output written to <file>' instead of the rendered content).

  All recoverable errors (field not found, missing draft, malformed foundation JSON) live in the success/error envelope. Only args_json parse failures escalate to FspecCoreError::InvalidArgs.

  Both invocation paths (LLM dispatcher and clap subcommand) call the single fspec_core::commands::show_foundation::run function; CLI bridge does only JSON arg marshalling and stdout rendering.

  The dispatcher payload shape is `{ section: Option<String>, format: Option<String>, output: Option<String>, draft: Option<bool> }`. The clap variant exposes optional positional `[section]` + `--section <section>` + `--format <format>` + `--output <file>` + `--draft` + `--list-sections` + `--line-numbers` flags. --list-sections and --line-numbers are no-ops (parity with TS source which advertises them but does not implement them in showFoundationCommand).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/foundation.json via ensure_foundation_file when --draft is NOT set (auto-creates with canonical v2.0.0 default if missing)
  #   2. When --draft is set, reads spec/foundation.json.draft directly (NO auto-create); missing draft surfaces a 'No draft found ...' error in the envelope
  #   3. Section lookup first checks FIELD_MAP; otherwise treats section as a dotted JSON path
  #   4. Missing field path surfaces error "Field '<section>' not found"
  #   5. format='json' (and any non-text value) ALWAYS emits 2-space pretty-printed JSON
  #   6. format='text' with a section: string → raw string; otherwise → 2-space pretty-printed JSON
  #   7. format='text' with NO section: renders multi-line PROJECT / PROBLEM SPACE / SOLUTION SPACE / PERSONAS / ARCHITECTURE DIAGRAMS block
  #   8. When --output <file> is set, the formatted output is written to that file and the CLI prints '✓ Output written to <file>' instead of the content
  #   9. The CLI surface advertises --list-sections and --line-numbers flags but they are no-ops (TS parity)
  #   10. The CLI surface has one optional positional <section> argument plus --section, --format, --output, --draft, --list-sections, --line-numbers flags
  #   11. Default --format is 'text'; default --draft is false
  #   12. CLI exit code is 0 on success and 1 on any error
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of show-foundation wired through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one foundation display implementation

  Scenario: Returns text render with PROJECT PROBLEM SPACE SOLUTION SPACE and PERSONAS sections by default
    Given spec/foundation.json contains project.name='fspec', project.vision='V', project.projectType='cli-tool', a primary problem with title/description/impact, a solution overview with one capability, and one persona
    When I dispatch show-foundation with no section and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the exact line '=== PROJECT ==='
    Then the DispatchResult.data contains the line 'Name: fspec'
    Then the DispatchResult.data contains the line 'Vision: V'
    Then the DispatchResult.data contains the line 'Type: cli-tool'
    Then the DispatchResult.data contains the exact line '=== PROBLEM SPACE ==='
    Then the DispatchResult.data contains the exact line '=== SOLUTION SPACE ==='
    Then the DispatchResult.data contains the exact line '=== PERSONAS ==='

  Scenario: Returns the entire foundation as pretty-printed JSON when format is json
    Given spec/foundation.json contains a complete v2.0.0 foundation
    When I dispatch show-foundation with no section and format='json'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as JSON whose root has a 'project' field
    Then the DispatchResult.data uses 2-space indentation

  Scenario: Resolves projectName via FIELD_MAP and emits a raw string in text format
    Given spec/foundation.json contains project.name='fspec'
    When I dispatch show-foundation with section='projectName' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data equals exactly 'fspec'

  Scenario: Resolves projectName via FIELD_MAP and emits a JSON-quoted string in json format
    Given spec/foundation.json contains project.name='fspec'
    When I dispatch show-foundation with section='projectName' and format='json'
    Then the dispatcher returns success=true
    Then the DispatchResult.data equals exactly '"fspec"'

  Scenario: Section pointing to an object emits pretty-printed JSON in text format
    Given spec/foundation.json contains project.name='fspec' and project.projectType='cli-tool'
    When I dispatch show-foundation with section='project' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as JSON whose root has 'name' and 'projectType'
    Then the DispatchResult.data uses 2-space indentation

  Scenario: Missing section returns Field not found error
    Given spec/foundation.json contains a complete foundation
    When I dispatch show-foundation with section='nonexistent' and format='text'
    Then the dispatcher returns success=false with an error message exactly "Field 'nonexistent' not found"

  Scenario: Dotted path bypasses FIELD_MAP for unmapped sections
    Given spec/foundation.json contains project.name='fspec'
    When I dispatch show-foundation with section='project.name' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data equals exactly 'fspec'

  Scenario: draft=true with no draft file returns the canonical missing draft error
    Given spec/foundation.json.draft does NOT exist in the project root
    When I dispatch show-foundation with draft=true
    Then the dispatcher returns success=false with an error message exactly 'No draft found at spec/foundation.json.draft. Run `fspec discover-foundation` to create one.'

  Scenario: draft=true reads the draft file instead of foundation.json
    Given spec/foundation.json contains project.name='final-name'
    Given spec/foundation.json.draft contains project.name='draft-name'
    When I dispatch show-foundation with section='projectName' and draft=true and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data equals exactly 'draft-name'

  Scenario: Empty workspace auto-creates spec/foundation.json with canonical defaults
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch show-foundation with section='projectName' and format='text'
    Then the dispatcher returns success=true
    Then the DispatchResult.data equals exactly 'Project Name'
    Then spec/foundation.json exists after the call (auto-created by ensure_foundation_file)

  Scenario: Escalates malformed foundation.json as a structured parse error
    Given spec/foundation.json exists but contains the malformed bytes '{ not json'
    When I dispatch show-foundation against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse foundation.json'

  Scenario: output writes formatted content to disk and CLI prints success line
    Given spec/foundation.json contains project.name='fspec'
    When I dispatch show-foundation with section='projectName' and format='text' and output='out/name.txt'
    Then the dispatcher returns success=true
    Then the file <project_root>/out/name.txt exists with the exact bytes 'fspec'

  Scenario: Default format value is 'text' when format flag is omitted
    Given spec/foundation.json contains project.name='fspec'
    When I dispatch show-foundation with section='projectName' and no format flag
    Then the dispatcher returns success=true
    Then the DispatchResult.data equals exactly 'fspec'
