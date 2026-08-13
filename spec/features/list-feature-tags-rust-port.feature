@done
@rust
@cli
@RPC-244
Feature: Port list-feature-tags command to Rust
  """
  New impl file at rust/fspec-core/src/commands/list_feature_tags.rs replaces the NotYetPorted stub. The module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` with the same signature shape as list_features::run. Args struct deserializes `{file: String (required), showCategories?: bool, format?: 'text'|'json'}` with `#[serde(default)]` on the optional fields.
  Gherkin parsing reuses the inline line-scanner approach pioneered by list_features.rs — feature-level tags only (tag lines accumulated until a Feature: keyword is reached). Scenario-level tags are intentionally NOT returned (parity with TS `gherkinDocument.feature.tags.map(t => t.name)` at src/commands/list-feature-tags.ts:70).
  Category lookup (showCategories=true) reads spec/tags.json via the existing TagsData type at rust/fspec-core/src/types/tags.rs, building a tag→category map from `categories[*].tags[*]`. If the read or parse fails, the function silently degrades to returning tags without categories (parity with the bare TS catch at src/commands/list-feature-tags.ts:103-109). The tags.json file is NOT auto-created here — that is list-tags' job.
  Error semantics mirror the TS ListFeatureTagsResult shape exactly: ENOENT, invalid Gherkin syntax, and missing Feature header are all surfaced as `{success:false, tags:[], error:'<message>'}` in the structured result, NEVER escalated as FspecCoreError. Only args_json deserialisation failure escalates via FspecCoreError::InvalidArgs.
  Both invocation paths (LLM dispatcher and shell-facing CLI bridge) converge on this single `pub async fn run` — the CLI bridge module at rust/fspec/src/list_feature_tags.rs is delivered as part of RPC-244 itself and performs only JSON arg marshalling + CWD resolution; see spec/features/list-feature-tags-cli-subcommand.feature for the clap surface contract.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-feature-tags` MUST replace the NotYetPorted stub
  #   2. Missing feature file (ENOENT) → success=false with error 'File not found: <path>'
  #   3. Invalid Gherkin syntax → success=false with error containing 'Invalid Gherkin syntax'
  #   4. File without a Feature header → success=false with error 'File does not contain a valid Feature'
  #   5. Feature with no tags → success=true, tags=[], message='No tags found on this feature'
  #   6. Feature with tags → success=true, tags array in declaration order, NO message field
  #   7. Only feature-level tags are returned — scenario-level tags are NOT included
  #   8. Tag names retain their leading '@' prefix on output
  #   9. showCategories=true with registered tags → categorizedTags array of {tag, category}
  #   10. showCategories=true with unregistered tag → category='Unknown' for that tag
  #   11. showCategories=true with missing/unreadable tags.json → silently degrades (returns tags without categorizedTags)
  #   12. JSON format emits 2-space indented payload with canonical field order
  #   13. CLI surface: <file> positional arg + --show-categories flag (parity with Commander.js)
  #   14. Two-front-doors invariant — LLM dispatcher and CLI bridge call the same pub async fn run
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-feature-tags from the agent loop AND invoke `fspec list-feature-tags <file>` from a shell
    So that I can audit the top-level tags on a single feature file (with optional category cross-reference) without going through Node.js

  Scenario: Returns error when the requested feature file does not exist
    Given an empty project root directory containing no spec/features/missing.feature
    When I dispatch list-feature-tags with file='spec/features/missing.feature' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has success=false
    Then the parsed JSON has tags array of length 0
    Then the parsed JSON has error field equal to 'File not found: spec/features/missing.feature'

  Scenario: Returns feature-level tags in declaration order when feature has tags
    Given spec/features/user-auth.feature exists with feature-level tags '@critical @auth @wip' on a single line before 'Feature: User Authentication'
    When I dispatch list-feature-tags with file='spec/features/user-auth.feature' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has success=true
    Then the parsed JSON has tags=['@critical','@auth','@wip'] in that exact order
    Then the parsed JSON does NOT contain a top-level 'message' field
    Then the parsed JSON does NOT contain a top-level 'error' field

  Scenario: Returns empty tags with sentinel message when feature has no tags
    Given spec/features/no-tags.feature exists containing 'Feature: No Tags' with no tag lines anywhere in the file
    When I dispatch list-feature-tags with file='spec/features/no-tags.feature' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has success=true
    Then the parsed JSON has tags array of length 0
    Then the parsed JSON has message field equal to 'No tags found on this feature'

  Scenario: Excludes scenario-level tags from the returned tag list
    Given spec/features/mixed-tags.feature exists with feature-level tag '@critical' and a scenario tagged '@smoke' beneath the Feature header
    When I dispatch list-feature-tags with file='spec/features/mixed-tags.feature' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has success=true
    Then the parsed JSON has tags=['@critical'] containing exactly one entry
    Then the tags array does NOT contain '@smoke'

  Scenario: Returns error when file does not contain a valid Feature header
    Given spec/features/junk.feature exists containing only the bytes 'This is not gherkin at all\n'
    When I dispatch list-feature-tags with file='spec/features/junk.feature' and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has success=false
    Then the parsed JSON has tags array of length 0
    Then the parsed JSON has error field equal to 'File does not contain a valid Feature'

  Scenario: Pairs each tag with its category when showCategories=true and tags are registered
    Given spec/features/critical.feature exists with feature-level tag '@critical'
    Given spec/tags.json registers '@critical' under category 'Priority Tags'
    When I dispatch list-feature-tags with file='spec/features/critical.feature', showCategories=true, and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has success=true
    Then the parsed JSON has tags=['@critical']
    Then the parsed JSON has categorizedTags=[{tag:'@critical',category:'Priority Tags'}]

  Scenario: Maps unregistered tags to category 'Unknown' when showCategories=true
    Given spec/features/exotic.feature exists with feature-level tag '@nonexistent'
    Given spec/tags.json exists but does NOT register '@nonexistent' in any category
    When I dispatch list-feature-tags with file='spec/features/exotic.feature', showCategories=true, and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has success=true
    Then the parsed JSON has tags=['@nonexistent']
    Then the categorizedTags array contains exactly one entry with tag='@nonexistent' and category='Unknown'

  Scenario: Silently degrades to plain tags when showCategories=true but tags.json is missing
    Given spec/features/simple.feature exists with feature-level tag '@critical'
    Given the project root contains no spec/tags.json
    When I dispatch list-feature-tags with file='spec/features/simple.feature', showCategories=true, and format='json'
    Then the dispatcher returns success=true
    Then the parsed JSON has success=true
    Then the parsed JSON has tags=['@critical']
    Then the parsed JSON does NOT contain a top-level 'categorizedTags' field
    Then the parsed JSON does NOT contain a top-level 'error' field

  Scenario: JSON format emits two-space indented payload
    Given spec/features/tagged.feature exists with feature-level tag '@critical'
    When I dispatch list-feature-tags with file='spec/features/tagged.feature' and format='json'
    Then the DispatchResult.data starts with the exact string "{\n  \"success\": true,\n"
    Then the DispatchResult.data contains the exact substring "\"tags\": [\n    \"@critical\"\n  ]"

  Scenario: Text format renders the populated case as a bullet list under a header
    Given spec/features/tagged.feature exists with feature-level tags '@critical' and '@auth'
    When I dispatch list-feature-tags with file='spec/features/tagged.feature' and format='text'
    Then the DispatchResult.data contains the line 'Tags on this feature:'
    Then the DispatchResult.data contains the exact line '  @critical'
    Then the DispatchResult.data contains the exact line '  @auth'

  Scenario: Text format prints sentinel message when feature has no tags
    Given spec/features/no-tags.feature exists containing 'Feature: No Tags' with no tag lines
    When I dispatch list-feature-tags with file='spec/features/no-tags.feature' and format='text'
    Then the DispatchResult.data is exactly the string 'No tags found on this feature'

  Scenario: Default format (no format key supplied) is text
    Given spec/features/tagged.feature exists with feature-level tag '@critical'
    When I dispatch list-feature-tags with file='spec/features/tagged.feature' and no format key in the args object
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the line 'Tags on this feature:'
    Then the DispatchResult.data contains the exact line '  @critical'
