@done
@rust
@cli
@RPC-249
Feature: Port list-scenario-tags command to Rust
  """
  New impl file at rust/fspec-core/src/commands/list_scenario_tags.rs replaces the NotYetPorted stub. The module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` with the same signature shape as list_hooks::run. Args struct deserializes `{file: string, scenario: string, showCategories?: boolean, format?: 'text'|'json'}` with `#[serde(default)]` on optional fields.
  Feature parsing follows the inline-scanner approach established by list_features.rs (RPC-245) rather than depending on the upstream `gherkin` crate (which is NOT in the workspace today and would also require re-prepending the '@' prefix per gherkin-porting-guide.md §3). The inline scanner extracts (a) the Feature block presence, (b) each top-level Scenario header line text, and (c) the most-recently-accumulated `@tag` block immediately preceding that Scenario header. Background/Examples/Rule:Scenario children are ignored (parity with TS `child.scenario.keyword === 'Scenario'` filter).
  Error semantics (parity with src/commands/list-scenario-tags.ts:30-86):
  - ENOENT on the feature file → success=false with error 'File not found: <path>'
  - Other I/O errors → success=false escalated via FspecCoreError::Io
  - Parse failure → success=false with error 'Invalid Gherkin syntax: <reason>' (inline scanner reports a structural error when no Feature: header is found and no Scenario lines either — happy enough for the smoke tests; a real malformed-source like an unterminated docstring is reported with a synthetic message)
  - No Feature element → success=false with error 'File does not contain a valid Feature'
  - Scenario not found → success=false with error "Scenario '<name>' not found in <path>"
  - Tags empty → success=true with empty tags and message 'No tags found on this scenario'
  - showCategories with missing/invalid spec/tags.json → graceful degrade (drop categorizedTags field, keep tags)
  Output: JSON format wraps the result in a `{success, tags, message?, error?, categorizedTags?}` object with 2-space indent; text format renders a human-readable listing identical to the TS CLI wrapper output.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Dispatcher route for `list-scenario-tags` MUST replace the NotYetPorted stub
  #   2. ENOENT on the feature file → success=false with error 'File not found: <path>'
  #   3. Other I/O errors → success=false with FspecCoreError::Io message
  #   4. Malformed Gherkin → success=false with error starting 'Invalid Gherkin syntax:'
  #   5. No Feature element in file → success=false with error 'File does not contain a valid Feature'
  #   6. Scenario name not found → success=false with error "Scenario '<name>' not found in <file>"
  #   7. Scenario found with zero tags → success=true, tags=[], message='No tags found on this scenario'
  #   8. Scenario found with tags → success=true, tags=['@tag1', ...] (leading '@' preserved)
  #   9. Scenario name match is exact and case-sensitive
  #   10. Only top-level Scenario children are searched (Background and Rule:Scenario excluded)
  #   11. showCategories=true with valid spec/tags.json → adds categorizedTags array
  #   12. showCategories=true with unknown tag → category='Unknown'
  #   13. showCategories=true with missing/invalid spec/tags.json → success=true, tags returned, NO categorizedTags
  #   14. JSON format = 2-space indented {success, tags, ...}; default format is text
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-scenario-tags from the agent loop AND invoke `fspec list-scenario-tags` from a shell
    So that I can audit the tags on a specific Gherkin scenario, optionally enriched by the project tag registry, sharing one source-of-truth between the LLM dispatcher and the CLI without going through Node.js

  Scenario: Returns 'File not found' error when feature file does not exist
    Given an empty project root directory with no spec/features/ subdirectory
    When I dispatch list-scenario-tags with file='spec/features/nope.feature' and scenario='Anything' and format='json'
    Then the dispatcher result has DispatchResult.success=true (no FspecCoreError envelope) and DispatchResult.data parses to a JSON object with success=false, tags=[], and error='File not found: spec/features/nope.feature'

  Scenario: Returns 'Invalid Gherkin syntax' error when feature file is malformed
    Given the project root contains spec/features/broken.feature whose content is a Scenario keyword line with no preceding Feature header
    When I dispatch list-scenario-tags with file='spec/features/broken.feature' and scenario='Anything' and format='json'
    Then DispatchResult.data parses to JSON with success=false, tags=[], and error starting with 'Invalid Gherkin syntax:'

  Scenario: Returns 'Scenario not found' error when scenario name is absent
    Given the project root contains spec/features/login.feature with a Feature header 'User Login' and a single Scenario 'Login with valid credentials'
    When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Nope' and format='json'
    Then DispatchResult.data parses to JSON with success=false, tags=[], and error exactly equal to "Scenario 'Nope' not found in spec/features/login.feature"

  Scenario: Returns success with empty tags and sentinel message when scenario has no tags
    Given the project root contains spec/features/login.feature with a Feature header 'User Login' and a Scenario 'Untagged Scenario' that has NO @-tag lines preceding it
    When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Untagged Scenario' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has tags array of length 0
    Then the parsed JSON has message field equal to 'No tags found on this scenario'

  Scenario: Returns tags array with leading '@' preserved when scenario has tags
    Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' immediately preceded by tag line '@smoke @critical'
    When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON tags array equals ['@smoke','@critical']
    Then the parsed JSON does NOT contain a top-level 'message' field

  Scenario: Only matches top-level Scenario keyword (Background and Rule:Scenario excluded)
    Given the project root contains spec/features/login.feature whose Background block is named 'Login with valid credentials' and whose only top-level Scenario is named 'Other'
    When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and format='json'
    Then DispatchResult.data parses to JSON with success=false and error containing the substring "Scenario 'Login with valid credentials' not found"

  Scenario: showCategories enriches tags with category labels from spec/tags.json
    Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' tagged '@smoke'
    Given the project root contains spec/tags.json with a category 'Testing Tags' whose tags include {name:'@smoke'}
    When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and showCategories=true and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON categorizedTags array contains exactly one entry with tag='@smoke' and category='Testing Tags'

  Scenario: showCategories labels tags absent from the registry as 'Unknown'
    Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' tagged '@custom'
    Given the project root contains spec/tags.json with a category 'Testing Tags' whose tags include {name:'@smoke'} (no '@custom')
    When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and showCategories=true and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON categorizedTags array contains exactly one entry with tag='@custom' and category='Unknown'

  Scenario: showCategories degrades gracefully when spec/tags.json is missing
    Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' tagged '@smoke'
    Given the project root has NO spec/tags.json file
    When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and showCategories=true and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON tags array equals ['@smoke']
    Then the parsed JSON does NOT contain a top-level 'categorizedTags' field

  Scenario: showCategories degrades gracefully when spec/tags.json is invalid JSON
    Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' tagged '@smoke'
    Given the project root contains spec/tags.json with the malformed bytes '{ not json'
    When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and showCategories=true and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON tags array equals ['@smoke']
    Then the parsed JSON does NOT contain a top-level 'categorizedTags' field

  Scenario: Excludes Scenarios nested inside Rule: blocks
    Given the project root contains spec/features/rules.feature with a Feature header and a Rule block named 'AuthRule' whose nested Scenario is 'Login with valid credentials'
    When I dispatch list-scenario-tags with file='spec/features/rules.feature' and scenario='Login with valid credentials' and format='json'
    Then DispatchResult.data parses to JSON with success=false and error containing the substring "Scenario 'Login with valid credentials' not found"

  Scenario: Excludes Scenario Outline keyword (only matches plain Scenario)
    Given the project root contains spec/features/outline.feature with a Feature header and a single 'Scenario Outline: Login with valid credentials' header (no plain Scenario by that name)
    When I dispatch list-scenario-tags with file='spec/features/outline.feature' and scenario='Login with valid credentials' and format='json'
    Then DispatchResult.data parses to JSON with success=false and error containing the substring "Scenario 'Login with valid credentials' not found"
