@done
@documentation
@cli
@RPC-299
Feature: Port show-acceptance-criteria command to Rust
  """
  Uses gherkin crate (already in fspec-core deps) to parse feature files. Pattern matches show_feature.rs but iterates the entire spec/features/**/*.feature glob via glob_feature_files (already shared). Children filter is keyword == 'Scenario' — Scenario Outlines are intentionally skipped (TS parity at show-acceptance-criteria.ts:146).
  Three rendering paths: text (ANSI-color compat via plain text — non-TTY identity), markdown (H1/H2/bullet steps with blockquote background), json (serde_json::to_string_pretty with 2-space indent over the FeatureAC vec). Order of features matches glob_feature_files (sorted ascending by path).
  Dispatcher returns full {success, features, totalScenarios, message, output} envelope as 2-space JSON; CLI bridge reads result.output (formatted body) and result.message and prints message + (optionally) output to stdout. Output path resolution: TS uses raw options.output (relative to process.cwd) — Rust mirrors by joining project_root if not absolute.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch show-acceptance-criteria to render scenarios from feature files filtered by tag in text/markdown/json format with optional output to file
    So that I can extract acceptance criteria documentation from spec/features/ without launching Node, sharing one Rust source of truth between the LLM dispatcher and the CLI

  Scenario: Missing spec/features directory returns a structured error
    Given an empty temp project root with no spec/ subdirectory
    When I dispatch show-acceptance-criteria with no arguments
    Then the dispatcher returns success=false
    And the error field contains the substring 'spec/features directory not found'

  Scenario: Empty spec/features directory returns success with message
    Given a temp project root with an empty spec/features/ directory
    When I dispatch show-acceptance-criteria with no arguments
    Then the dispatcher returns success=true
    And the data.features array is empty
    And the data.message equals 'No feature files found in spec/features/'

  Scenario: Tag filter selects only features carrying that tag
    Given a temp project root with two feature files - 'login.feature' tagged '@auth' and 'misc.feature' tagged '@misc'
    When I dispatch show-acceptance-criteria with tags=['@auth']
    Then the dispatcher returns success=true
    And the data.features array has 1 element
    And the data.features[0].tags contains '@auth'

  Scenario: Multiple tag filters require ALL tags present
    Given a temp project root with one feature tagged '@critical @auth' and one tagged '@critical' only
    When I dispatch show-acceptance-criteria with tags=['@critical','@auth']
    Then the dispatcher returns success=true
    And the data.features array has 1 element
    And the data.features[0].tags contains both '@critical' and '@auth'

  Scenario: format=markdown renders H1, blockquote, H2, and bullet steps
    Given a temp project root with one feature 'login.feature' tagged '@auth' containing a background and one scenario with steps
    When I dispatch show-acceptance-criteria with tags=['@auth'] and format='markdown'
    Then the dispatcher returns success=true
    And the data.output contains the substring '# '
    And the data.output contains the substring '## '
    And the data.output contains the substring '- **'

  Scenario: format=json renders 2-space JSON array with name, tags, description, scenarios
    Given a temp project root with one feature 'login.feature' tagged '@auth' with one scenario
    When I dispatch show-acceptance-criteria with tags=['@auth'] and format='json'
    Then the dispatcher returns success=true
    And the data.output parses as a JSON array
    And the first element has name, tags, and scenarios properties
    And the data.output uses 2-space indentation

  Scenario: Tag matches zero features returns 'No features found matching tags' message
    Given a temp project root with one feature 'misc.feature' tagged '@misc'
    When I dispatch show-acceptance-criteria with tags=['@deprecated']
    Then the dispatcher returns success=true
    And the data.features array is empty
    And the data.message equals 'No features found matching tags: @deprecated'

  Scenario: Feature without Background section is rendered without background block
    Given a temp project root with one feature 'noback.feature' tagged '@test' that has no Background section
    When I dispatch show-acceptance-criteria with tags=['@test']
    Then the dispatcher returns success=true
    And the data.features[0].background is null

  Scenario: Feature with no scenarios shows 'No scenarios defined' marker in text and markdown
    Given a temp project root with one feature 'empty.feature' tagged '@empty' having no scenarios
    When I dispatch show-acceptance-criteria with tags=['@empty'] and format='markdown'
    Then the dispatcher returns success=true
    And the data.output contains the substring '_No scenarios defined_'

  Scenario: Output path writes rendered content to disk and changes the message
    Given a temp project root with one feature 'login.feature' tagged '@auth' with one scenario
    When I dispatch show-acceptance-criteria with tags=['@auth'], format='markdown', and output='out/acs.md'
    Then the dispatcher returns success=true
    And the file <project_root>/out/acs.md exists with the same bytes as the formatted markdown
    And the data.message equals 'Acceptance criteria written to acs.md'

  Scenario: Summary line reports scenario and feature counts
    Given a temp project root with three feature files all tagged '@critical' having 15 total scenarios
    When I dispatch show-acceptance-criteria with tags=['@critical']
    Then the dispatcher returns success=true
    And the data.totalScenarios equals 15
    And the data.message contains the substring 'Showing acceptance criteria for 15 scenarios from 3 features'

  Scenario: Shared infrastructure module is registered for show-acceptance-criteria
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/commands/show_acceptance_criteria.rs
    Then the module no longer returns FspecCoreError::NotYetPorted
    And the dispatcher routes show-acceptance-criteria to the new run function
