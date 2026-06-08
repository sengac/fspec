@feature-management
@cli
@done
@RPC-304 @wip
Feature: Port show-feature command to Rust

  """
  Reuses the workspace `gherkin = 0.16` crate (already used by list_scenario_tags) for AST parsing. The Rust gherkin crate does NOT produce @cucumber/gherkin-style UUIDs or column positions, so JSON output is a Rust-shaped projection (feature.name, feature.tags, feature.children/scenario steps, location.line) — NOT byte-for-byte parity with the TS JSON AST.
  Bare-name lookup reuses io::feature_glob::glob_feature_files (sorted, forward-slash paths) and picks the first whose basename minus '.feature' equals the input. Direct path lookup does NOT use glob — it joins project_root + input and stat-checks.
  Work-unit tag pattern is the regex ^@([A-Z]{2,6}-\d+)$ — uppercase only, 2-6 letter prefix, dash, digits. Implemented as a small parser without pulling regex into fspec-core's runtime deps.
  Dispatcher front door resolves `output` path relative to project_root; CLI bridge resolves `output` path relative to env::current_dir(). Both write via std::fs::write.
  TS-parity result envelope is `{success, content?, format?, validated?, error?, workUnits?}`. All recoverable errors (feature-not-found, invalid gherkin, I/O) live in the envelope — only args_json parse failures escalate to FspecCoreError::InvalidArgs.
  """

  Background: User Story
    As a fspec maintainer porting the show-feature command to Rust
    I want to have a single Rust function (and its dual CLI/dispatcher front doors) that renders a feature file's contents with work-unit annotations, matching the TypeScript source's behaviour
    So that the Rust agent loop no longer hangs on the show-feature stub and shell users see byte-for-byte parity

  Scenario: Bare-name lookup resolves to spec/features/<name>.feature and renders Work Units None when no WU tags present
    Given a temp project root contains spec/features/login.feature with valid gherkin and no @PREFIX-NNN tags
    When I dispatch show-feature with feature='login' and format='text'
    Then the dispatcher returns success=true
    And the data field reproduces the file content verbatim followed by '\n\nWork Units: None\n'

  Scenario: Direct path lookup resolves a feature path ending in .feature
    Given a temp project root contains spec/features/login.feature with valid gherkin and no @PREFIX-NNN tags
    When I dispatch show-feature with feature='spec/features/login.feature' and format='text'
    Then the dispatcher returns success=true
    And the data field equals the data returned for the bare-name lookup with feature='login'

  Scenario: Missing feature file returns Feature file not found with the unresolved input
    Given a temp project root with no spec/features/ directory
    When I dispatch show-feature with feature='missing-name' and format='text'
    Then the dispatcher returns success=false
    And the error field equals 'Feature file not found: missing-name'

  Scenario: Invalid Gherkin syntax returns success false with Invalid Gherkin syntax prefix
    Given a temp project root contains spec/features/broken.feature with the bytes 'this is not gherkin'
    When I dispatch show-feature with feature='broken' and format='text'
    Then the dispatcher returns success=false
    And the error field starts with the prefix 'Invalid Gherkin syntax: '

  Scenario: Feature-level work-unit tag attaches to scenarios that lack their own WU tag
    Given a temp project root contains spec/features/auth.feature tagged '@AUTH-001' at the feature level with two scenarios 'A' on line 4 and 'B' on line 7 with no scenario-level work-unit tags
    And spec/work-units.json contains AUTH-001 with title 'Login' and status 'implementing'
    When I dispatch show-feature with feature='auth' and format='text'
    Then the dispatcher returns success=true
    And the data field contains the substring '\nWork Units:\n'
    And the data field contains the exact line '  AUTH-001 (feature-level) - Login'
    And the data field contains the exact line '    auth.feature:4 - A'
    And the data field contains the exact line '    auth.feature:7 - B'

  Scenario: Scenario-level WU tag overrides feature-level for that scenario and produces a scenario-level entry
    Given a temp project root contains spec/features/mixed.feature tagged '@AUTH-001' at the feature level with scenario 'X' additionally tagged '@AUTH-002' and scenario 'Y' having no scenario-level tag
    And spec/work-units.json contains AUTH-001 with title 'Login' and status 'done' and AUTH-002 with title 'Logout' and status 'implementing'
    When I dispatch show-feature with feature='mixed' and format='text'
    Then the dispatcher returns success=true
    And the AUTH-001 block in the data field has level 'feature-level' and lists only scenario 'Y'
    And the AUTH-002 block in the data field has level 'scenario-level' and lists only scenario 'X'

  Scenario: Missing work-units json yields Unknown title and unknown status enrichment
    Given a temp project root contains spec/features/orphan.feature tagged '@AUTH-001' at the feature level with one scenario 'A'
    And spec/work-units.json does NOT exist
    When I dispatch show-feature with feature='orphan' and format='text'
    Then the dispatcher returns success=true
    And the data field contains the exact line '  AUTH-001 (feature-level) - Unknown'

  Scenario: JSON format emits a 2-space-indented object with feature AST and workUnits array
    Given a temp project root contains spec/features/login.feature with one scenario 'A' and no work-unit tags
    When I dispatch show-feature with feature='login' and format='json'
    Then the dispatcher returns success=true
    And the data field parses as JSON whose root object has a 'feature' field containing 'name' and 'children'
    And the data field parses as JSON whose root object has a 'workUnits' array (empty when no tags present)
    And the data field uses 2-space indentation

  Scenario: Output path writes rendered content to disk and content field still echoes it
    Given a temp project root contains spec/features/login.feature with valid gherkin and no work-unit tags
    When I dispatch show-feature with feature='login', format='text', and output='out/snapshot.txt'
    Then the dispatcher returns success=true
    And the file <project_root>/out/snapshot.txt exists with the same bytes as the data field

  Scenario: Shared infrastructure modules exist under fspec-core for reuse by other commands
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/src/
    Then the helper io::feature_glob::glob_feature_files is publicly accessible from the crate root
    And commands/show_feature.rs no longer returns FspecCoreError::NotYetPorted
