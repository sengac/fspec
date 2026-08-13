@done
@RPC-316
Feature: Port update-tag command to Rust
  """
  Files: rust/fspec-core/src/commands/update_tag.rs (replace stub); rust/fspec-core/src/help/configs/update_tag.rs (NEW help config); rust/fspec/src/update_tag.rs (NEW CLI bridge); rust/fspec-core/tests/update_tag.rs (NEW dispatcher tests); rust/fspec/tests/cli_update_tag.rs (NEW CLI shell tests); rust/fspec/tests/fixtures/help/update-tag.txt (NEW captured fixture)
  Reuses shared infrastructure: io::ensure (READ only — load tags.json via direct fs::read; NO auto-create), io::locked_file::write_json_atomic for atomic write, types::tags::{TagsData, TagCategory, Tag} with #[serde(flatten)] extra preserving aux fields
  TAGS.md regeneration: inline minimal generate_tags_md helper duplicated from register_tag.rs (same shape: warning header, ## Tag Categories with per-category tables, optional _Last updated_ line). Will be promoted to shared generators module when delete_tag lands (also Batch 7).
  Divergences from TS (documented in code): (1) Ajv schema validation omitted — upstream gates enforce schema invariants; (2) outer try/catch flattened to direct FspecCoreError returns; (3) statistics.lastUpdated explicitly NOT bumped (matches TS behavior — only register-tag bumps it).
  Two-front-doors: dispatcher and clap CLI both call commands::update_tag::run(args_json, project_root). CLI bridge marshals positional <tag> + --category + --description into JSON object {tag, category, description}. NO logic in bridge — JSON marshalling only.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. At least one of --category or --description must be provided; otherwise fail with 'No updates specified. Use --category and/or --description'
  #   2. spec/tags.json must already exist; update-tag does NOT auto-create the file (opposite of register-tag). Missing file → 'spec/tags.json not found' error
  #   3. Tag lookup is exact-match, case-sensitive across ALL categories using findIndex on the tag-name string
  #   4. Tag-not-found error format: 'Tag {tag} not found in registry' with no suggestion list
  #   5. When --category is provided, target category lookup is case-sensitive exact-match (c.name === category) — unlike register-tag which uses case-insensitive lookup
  #   6. Unknown-category error format: 'Invalid category: {category}. Available categories: {csv-of-all-category-names-in-insertion-order}'
  #   7. Cross-category move: splice from current, push to target with provided description (or fallback to original tag's description if --description omitted), alphabetically sort target category after insert
  #   8. Description-only update (no --category, OR --category matching current category name): mutate description in place; do NOT re-sort the array
  #   9. statistics.lastUpdated is NOT bumped by update-tag (unlike register-tag) — TS code has no lastUpdated mutation. Auxiliary top-level fields round-trip untouched
  #   10. Atomic write to spec/tags.json via write_json_atomic; then regenerate spec/TAGS.md from in-memory data
  #   11. CLI surface: positional <tag> (required), --category <category> (optional), --description <description> (optional)
  #   12. Success message: '✓ Successfully updated {tag}' followed by '  Updated: spec/tags.json' and '  Regenerated: spec/TAGS.md' trailing lines
  #   13. Malformed tags.json (parse failure) returns a structured error — tags.json content unchanged on failure
  #
  # EXAMPLES:
  #   1. Update only description in same category: tag '@critical' in Phase Tags with new description 'Critical paths' — array order is preserved
  #   2. Move tag to different category: tag '@critical' moves from Phase Tags to Priority Tags, preserving its description, and Priority Tags is alphabetically sorted after insert
  #   3. Move tag with new description: tag '@critical' moves to Priority Tags AND description is updated to 'High priority' — both changes applied atomically
  #   4. Reject when neither --category nor --description provided: returns 'No updates specified' error and tags.json unchanged
  #   5. Reject when spec/tags.json missing: returns 'spec/tags.json not found' error
  #   6. Reject unknown tag: '@nonexistent' returns 'Tag @nonexistent not found in registry'
  #   7. Reject unknown target category 'Nonexistent Tags': returns 'Invalid category: Nonexistent Tags. Available categories: Phase Tags, Component Tags, ...' 
  #   8. Category lookup is case-sensitive: --category 'phase tags' (lowercase) does NOT match 'Phase Tags' — returns Invalid category error (divergence from register-tag behavior)
  #   9. Auxiliary fields preserved: combinationExamples, usageGuidelines, references round-trip untouched; statistics.lastUpdated is NOT bumped (unlike register-tag)
  #   10. Malformed tags.json: parse failure returns structured error; file on disk unchanged
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch update-tag from the agent loop with byte-for-byte parity to the TypeScript implementation
    So that I can refine the tag vocabulary (rename description, move category) without relying on Node.js, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Updates only the description when only --description is provided
    Given spec/tags.json contains a tag '@critical' under Phase Tags with description 'Critical features'
    When I dispatch update-tag with tag '@critical' and description 'Critical paths only'
    Then the dispatcher returns success=true
    And the Phase Tags category on disk contains a tag with name '@critical' and description 'Critical paths only'
    And the dispatcher output contains the substring 'Successfully updated @critical'

  Scenario: Moves tag to a different category preserving original description when --description is omitted
    Given spec/tags.json contains a tag '@critical' under Phase Tags with description 'Critical features'
    When I dispatch update-tag with tag '@critical' and category 'Priority Tags'
    Then the dispatcher returns success=true
    And Priority Tags exists as an empty category
    And the Phase Tags category on disk does not contain a tag named '@critical'
    And the Priority Tags category on disk contains a tag with name '@critical' and description 'Critical features'

  Scenario: Moves tag to a different category and overrides description when both --category and --description are provided
    Given spec/tags.json contains a tag '@critical' under Phase Tags with description 'Critical features'
    When I dispatch update-tag with tag '@critical', category 'Priority Tags', and description 'High priority work'
    Then the dispatcher returns success=true
    And Priority Tags exists as an empty category
    And the Priority Tags category on disk contains a tag with name '@critical' and description 'High priority work'

  Scenario: Sorts tags alphabetically within the target category after cross-category move
    Given spec/tags.json contains Phase Tags with tag '@critical' and Priority Tags with tags '@zed', '@aaa', '@mid' in that insertion order
    When I dispatch update-tag with tag '@critical' and category 'Priority Tags'
    Then the dispatcher returns success=true
    And the Priority Tags category on disk contains tags in the order '@aaa', '@critical', '@mid', '@zed'

  Scenario: Preserves insertion order when description-only update inside the same category
    Given spec/tags.json contains Phase Tags with tags '@zed', '@aaa', '@mid' in that insertion order
    When I dispatch update-tag with tag '@aaa' and description 'New A description'
    Then the dispatcher returns success=true
    And the Phase Tags category on disk contains tags in the order '@zed', '@aaa', '@mid'

  Scenario: Rejects request when neither --category nor --description is provided
    Given spec/tags.json contains a tag '@critical' under Phase Tags
    When I dispatch update-tag with tag '@critical' and no category or description
    Then the dispatcher returns success=false
    And the error message contains the substring 'No updates specified. Use --category and/or --description'
    And spec/tags.json content on disk is unchanged from before the call

  Scenario: Rejects request when spec/tags.json does not exist
    Given an empty project root directory with no spec/tags.json
    When I dispatch update-tag with tag '@critical' and description 'New description'
    Then the dispatcher returns success=false
    And the error message contains the substring 'spec/tags.json not found'
    And spec/tags.json was not created by the command

  Scenario: Rejects request when the tag is not found in any category
    Given spec/tags.json exists with the canonical empty category set
    When I dispatch update-tag with tag '@nonexistent' and description 'Anything'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Tag @nonexistent not found in registry'

  Scenario: Rejects unknown target category with canonical Available categories list
    Given spec/tags.json contains a tag '@critical' under Phase Tags
    When I dispatch update-tag with tag '@critical' and category 'Nonexistent Tags'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid category: Nonexistent Tags. Available categories: Phase Tags'
    And spec/tags.json content on disk is unchanged from before the call

  Scenario: Treats category lookup as case-sensitive (lowercase variant does not match)
    Given spec/tags.json contains a tag '@critical' under Phase Tags
    When I dispatch update-tag with tag '@critical' and category 'phase tags'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid category: phase tags'

  Scenario: Preserves auxiliary top-level fields and does NOT bump statistics.lastUpdated
    Given spec/tags.json contains a tag '@critical' under Phase Tags plus auxiliary fields combinationExamples, usageGuidelines, references, and statistics.lastUpdated set to '1999-01-01T00:00:00.000Z'
    When I dispatch update-tag with tag '@critical' and description 'New description'
    Then the dispatcher returns success=true
    And spec/tags.json on disk still contains combinationExamples, usageGuidelines, and references with their original payloads
    And spec/tags.json statistics.lastUpdated on disk still equals '1999-01-01T00:00:00.000Z'

  Scenario: Escalates malformed tags.json as a structured parse error
    Given spec/tags.json exists but contains invalid JSON syntax
    When I dispatch update-tag with tag '@critical' and description 'New description'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Failed to parse tags.json'
    And spec/tags.json content on disk is unchanged from before the call

  Scenario: Renders multi-line success block on success
    Given spec/tags.json contains a tag '@critical' under Phase Tags with description 'Critical features'
    When I dispatch update-tag with tag '@critical' and description 'Critical paths only'
    Then the dispatcher output contains the substring '✓ Successfully updated @critical'
    And the dispatcher output contains the substring 'Updated: spec/tags.json'
    And the dispatcher output contains the substring 'Regenerated: spec/TAGS.md'
