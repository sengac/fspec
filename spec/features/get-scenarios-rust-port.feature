@done
@querying
@cli
@RPC-237
Feature: Port get-scenarios command to Rust

  """
  Reuse io::feature_glob::glob_feature_files (sorted, forward-slash rel paths) for the spec/features walk and io::gherkin::parse_feature_lenient for parsing (same as show_acceptance_criteria.rs RPC-299). Missing spec/features dir: the TS access(featuresDir) check returns {success:false, error:'spec/features directory not found'}; in Rust escalate via FspecCoreError::Io with that exact substring so the dispatcher maps to success=false (mirror show_acceptance_criteria.rs).
  Result envelope is a serde struct {success, scenarios, totalCount, message, warnings?} in TS declaration order — use #[derive(Serialize)] not json!{} (BTreeMap alphabetizes). ScenarioInfo struct {feature, name, line, tags?} with #[serde(skip_serializing_if='Option::is_none')] tags. format=json prints ONLY the scenarios array (JSON.stringify(result.scenarios,null,2)), not the envelope; format=text builds the grouped human view. The dispatcher returns the full envelope; the CLI bridge picks the rendered string per format. Confirm exact dispatcher data shape against an already-ported gherkin read command (show_feature/show_acceptance_criteria) during testing.
  Framing A divergence: get-scenarios-help.ts documents a --file flag, but the actual TS Commander.js registration (registerGetScenariosCommand) only registers --tag (repeatable) and --format. The HELP FIXTURE is canon for byte-parity (so dependencies.txt/get-scenarios.txt are captured from node dist/index.js get-scenarios --help and WILL show --file), but the clap subcommand only needs to implement --tag and --format to match runtime behaviour. Mode::GetScenarios { tag: Vec<String>, format: Option<String> }; --tag repeatable (clap Vec), --format default 'text'. Bridge marshals {tags, format} omitting empties.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Globs spec/features/**/*.feature relative to project_root and parses each with the lenient gherkin parser; a missing spec/features directory returns a structured error whose message contains 'spec/features directory not found'
  #   2. Extracts only top-level scenarios whose keyword is exactly 'Scenario' (Scenario Outline and Rule-nested children handled per the gherkin crate AST); each emitted scenario carries feature (relative file path), name, line, and optional tags
  #   3. Tag filtering uses AND logic against the union of feature-level tags and the scenario's own tags (gherkin strips leading '@', restore it before comparing); when --tag is supplied, a scenario is included only if every requested tag is present in that union
  #   4. The scenario tags field reflects ONLY scenario-level tags (with '@' restored), and is None/omitted when the scenario has no own tags; inherited feature tags are used for matching but not stored on the scenario record
  #   5. Returns an envelope {success, scenarios, totalCount, message, warnings?}; message text mirrors TS exactly: 'No scenarios found matching tags: <tags>' / 'No scenarios found' / 'Found N scenario(s)' / 'Found N scenario(s) matching tags: <tags>' with pluralization on N==1; invalid feature files are skipped and recorded in warnings
  #   6. format json prints JSON.stringify(scenarios, null, 2) (the scenarios array only, NOT the envelope); format text (default) prints the message, a blank line, then scenarios grouped by feature with each scenario rendered as '  <line>: <name>[ [<tags>]]'; both invocation paths call the single fspec_core::commands::get_scenarios::run function
  #
  # EXAMPLES:
  #   1. Dispatch get-scenarios with format='json' and no tags against three feature files each with two scenarios returns success=true, totalCount=6, and a six-element scenarios array
  #   2. Dispatch get-scenarios with tags=['@auth','@smoke'] returns only scenarios whose feature+scenario tag union contains BOTH @auth and @smoke (AND logic)
  #   3. Dispatch get-scenarios with tags=['@deprecated'] when no feature carries that tag returns success=true, totalCount=0, message='No scenarios found matching tags: @deprecated'
  #   4. A scenario with scenario-level tags @smoke @critical is emitted with its tags field = ['@smoke','@critical'] while a scenario with no own tags has tags omitted (None)
  #   5. Dispatch get-scenarios against a project root with no spec/features directory returns success=false with an error message containing 'spec/features directory not found'
  #   6. CLI: `fspec get-scenarios --tag @auth --format json` prints a JSON array of scenario objects each with feature, name, line keys and exits 0
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to run `fspec get-scenarios` (optionally with repeatable --tag and --format text|json) through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one read-only scenario-extraction implementation with parity to the TS source

  Scenario: Dispatch with format json and no tags returns every scenario across all feature files
    Given a project root contains three feature files under spec/features/ each with two scenarios
    When I dispatch get-scenarios with format='json' and no tags
    Then the dispatcher returns success=true
    Then the returned envelope has totalCount=6
    Then the returned scenarios array has six elements

  Scenario: Dispatch with multiple tags applies AND logic against the feature plus scenario tag union
    Given a project root contains a feature tagged '@auth' with one scenario tagged '@smoke' and one scenario tagged '@regression'
    When I dispatch get-scenarios with tags=['@auth','@smoke']
    Then the dispatcher returns success=true
    Then the returned scenarios array contains only the scenario whose tag union includes both @auth and @smoke

  Scenario: Dispatch with a tag no feature carries returns a zero-count not-found message
    Given a project root contains feature files none of which carry the '@deprecated' tag
    When I dispatch get-scenarios with tags=['@deprecated']
    Then the dispatcher returns success=true
    Then the returned envelope has totalCount=0
    Then the returned envelope message equals 'No scenarios found matching tags: @deprecated'

  Scenario: Scenario-level tags are emitted while a scenario with no own tags omits the tags field
    Given a project root contains a feature whose first scenario is tagged '@smoke' and '@critical' and whose second scenario has no scenario-level tags
    When I dispatch get-scenarios with format='json' and no tags
    Then the dispatcher returns success=true
    Then the first scenario's tags field equals ['@smoke','@critical']
    Then the second scenario omits its tags field

  Scenario: Dispatch against a project root with no spec/features directory returns a structured not-found error
    Given a project root with no spec/features directory
    When I dispatch get-scenarios with no tags
    Then the dispatcher returns success=false with an error message containing the substring 'spec/features directory not found'
