@done
@rust
@cli
@RPC-246
Feature: Port list-foundation-sections command to Rust
  """
  New impl file at codelet/fspec-core/src/commands/list_foundation_sections.rs replaces the NotYetPorted stub. The module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` with the same signature shape as list_hooks::run, even though list-foundation-sections performs NO project root I/O (the section list is a static constant). Accepting `project_root` is parity with the dispatcher contract and lets the caller pass any path without affecting output.
  Args struct deserializes `{format?: 'text'|'json'}` with `#[serde(default)]`. The canonical section list is a `&'static [FoundationSectionSpec]` literal so the Rust port has zero allocations beyond serialization/rendering. Each spec carries `name`, `jsonPath`, `constraint`, optional `examples`, and `description`. We use `#[serde(skip_serializing_if = "<[&str]>::is_empty")]` on the `examples` field so sections without examples (every section EXCEPT projectType) omit the field from the JSON output, matching the TS optional `examples?: string[]` serialization.
  Text rendering mirrors `renderSectionsAsText` byte-for-byte: the header line 'Foundation Sections (update-foundation field reference)', a 57-character '=' separator line, a blank line, then for each section a bullet `• <name>` followed by `    path:       <jsonPath>`, `    constraint: <constraint>`, optionally `    examples:   <comma-joined>`, and `    about:      <description>`, with a blank line between sections. The closing footer is two lines about capabilities and personas being managed via dedicated commands.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-foundation-sections` MUST replace the NotYetPorted stub
  #   2. The section list is a static constant - no project root I/O is performed
  #   3. Exactly 7 sections are emitted in fixed order: projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview
  #   4. Each section carries name, jsonPath, constraint, description; examples is optional and present ONLY for projectType
  #   5. JSON format serializes the section array with 2-space indent and omits the examples field for sections that have none
  #   6. Text format begins with the exact header 'Foundation Sections (update-foundation field reference)' followed by '=' separator and blank line
  #   7. Each section row in text format emits four or five lines (name bullet, path:, constraint:, optional examples:, about:) followed by a blank line
  #   8. Text format ends with a two-line footer note about capabilities and personas being managed via dedicated commands
  #   9. Default format (no format key supplied) is text
  #   10. CLI surface is flag-less aside from --format (parity with TS Commander.js)
  #   11. Dispatch signature accepts project_root for parity but ignores it (no filesystem access)
  #   12. JSON jsonPath strings exactly match the TS source: project.name, project.vision, project.projectType, problemSpace.primaryProblem.title, problemSpace.primaryProblem.description, problemSpace.primaryProblem.impact, solutionSpace.overview
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-foundation-sections from the agent loop AND invoke `fspec list-foundation-sections` from a shell
    So that I can discover the canonical list of update-foundation field names with their JSON paths and constraints, sharing one source-of-truth between the LLM dispatcher and the CLI without going through Node.js

  Scenario: Default format (no format key supplied) is text
    Given an empty project root directory
    When I dispatch list-foundation-sections with an empty args object {}
    Then the dispatcher returns success=true
    And the DispatchResult.data starts with the exact line 'Foundation Sections (update-foundation field reference)'
    And the DispatchResult.data contains the exact line '========================================================='

  Scenario: JSON format returns exactly seven sections in canonical order
    Given an empty project root directory
    When I dispatch list-foundation-sections with format='json'
    Then the dispatcher returns success=true
    And the parsed JSON is an array of length 7
    And the entries have name values in order projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview

  Scenario: JSON format emits the canonical jsonPath strings for every section
    Given an empty project root directory
    When I dispatch list-foundation-sections with format='json'
    Then the dispatcher returns success=true
    And the projectName entry has jsonPath='project.name'
    And the projectVision entry has jsonPath='project.vision'
    And the projectType entry has jsonPath='project.projectType'
    And the problemTitle entry has jsonPath='problemSpace.primaryProblem.title'
    And the problemDefinition entry has jsonPath='problemSpace.primaryProblem.description'
    And the problemImpact entry has jsonPath='problemSpace.primaryProblem.impact'
    And the solutionOverview entry has jsonPath='solutionSpace.overview'

  Scenario: JSON format emits the canonical constraint strings for every section
    Given an empty project root directory
    When I dispatch list-foundation-sections with format='json'
    Then the dispatcher returns success=true
    And the projectName entry has constraint='freeform string'
    And the projectVision entry has constraint='freeform string'
    And the projectType entry has constraint='freeform string (1-30 characters)'
    And the problemTitle entry has constraint='freeform string'
    And the problemDefinition entry has constraint='freeform string'
    And the problemImpact entry has constraint='enum: high, medium, low'
    And the solutionOverview entry has constraint='freeform string'

  Scenario: JSON format omits the examples field for sections without examples
    Given an empty project root directory
    When I dispatch list-foundation-sections with format='json'
    Then the dispatcher returns success=true
    And the projectType entry has examples=['cli-tool','web-app','saas-platform']
    And the projectName entry does NOT contain a top-level 'examples' field
    And the projectVision entry does NOT contain a top-level 'examples' field
    And the problemTitle entry does NOT contain a top-level 'examples' field
    And the problemDefinition entry does NOT contain a top-level 'examples' field
    And the problemImpact entry does NOT contain a top-level 'examples' field
    And the solutionOverview entry does NOT contain a top-level 'examples' field

  Scenario: JSON format uses two-space indented pretty-printed payload
    Given an empty project root directory
    When I dispatch list-foundation-sections with format='json'
    Then the dispatcher returns success=true
    And the DispatchResult.data starts with the exact string "[\n  {\n    \"name\": \"projectName\""
    And the DispatchResult.data contains the exact substring "\"jsonPath\": \"project.name\""

  Scenario: Text format renders the header, separator, blank line, and seven section blocks
    Given an empty project root directory
    When I dispatch list-foundation-sections with format='text'
    Then the dispatcher returns success=true
    And the DispatchResult.data contains the exact line 'Foundation Sections (update-foundation field reference)'
    And the DispatchResult.data contains the exact line '========================================================='
    And the DispatchResult.data contains the exact line '• projectName'
    And the DispatchResult.data contains the exact line '• projectVision'
    And the DispatchResult.data contains the exact line '• projectType'
    And the DispatchResult.data contains the exact line '• problemTitle'
    And the DispatchResult.data contains the exact line '• problemDefinition'
    And the DispatchResult.data contains the exact line '• problemImpact'
    And the DispatchResult.data contains the exact line '• solutionOverview'
    And the substring '• projectName' appears before '• projectVision' in the output
    And the substring '• problemImpact' appears before '• solutionOverview' in the output

  Scenario: Text format renders path, constraint, and about lines for each section row
    Given an empty project root directory
    When I dispatch list-foundation-sections with format='text'
    Then the dispatcher returns success=true
    And the DispatchResult.data contains the exact line '    path:       project.name'
    And the DispatchResult.data contains the exact line '    constraint: freeform string'
    And the DispatchResult.data contains the exact line '    about:      Project name'
    And the DispatchResult.data contains the exact line '    path:       problemSpace.primaryProblem.impact'
    And the DispatchResult.data contains the exact line '    constraint: enum: high, medium, low'
    And the DispatchResult.data contains the exact line '    about:      How critical the problem is'

  Scenario: Text format renders the examples line only for projectType
    Given an empty project root directory
    When I dispatch list-foundation-sections with format='text'
    Then the dispatcher returns success=true
    And the DispatchResult.data contains the exact line '    examples:   cli-tool, web-app, saas-platform'
    And the DispatchResult.data contains exactly one line starting with '    examples:'

  Scenario: Text format ends with the two-line footer note about dedicated commands
    Given an empty project root directory
    When I dispatch list-foundation-sections with format='text'
    Then the dispatcher returns success=true
    And the DispatchResult.data contains the exact line 'Note: capabilities and personas are managed via dedicated commands'
    And the DispatchResult.data contains the exact line '      (add-capability, add-persona) and cannot be updated via update-foundation.'
    And the substring 'Note: capabilities and personas' appears after '• solutionOverview' in the output

  Scenario: Dispatch ignores the project_root path entirely
    Given a project root directory containing a populated spec/ with arbitrary contents
    When I dispatch list-foundation-sections with format='json'
    Then the dispatcher returns success=true
    And the parsed JSON is an array of length 7
    And the entries have name values in order projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview
