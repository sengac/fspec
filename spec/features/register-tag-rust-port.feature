@done
@RPC-265
Feature: Port register-tag command to Rust
  """
  Files: rust/fspec-core/src/commands/register_tag.rs (replace stub); rust/fspec-core/src/help/configs/register_tag.rs (NEW help config); rust/fspec/src/register_tag.rs (NEW CLI bridge); rust/fspec-core/tests/register_tag.rs (NEW dispatcher tests); rust/fspec/tests/cli_register_tag.rs (NEW CLI shell tests); rust/fspec/tests/fixtures/help/register-tag.txt (NEW captured fixture)
  Reuses existing shared infrastructure: io::ensure::ensure_tags_file (load-or-init), io::locked_file::write_json_atomic (atomic write), types::tags::{TagsData, TagCategory, Tag} with #[serde(flatten)] extra map preserving aux fields
  TAGS.md regeneration: inline minimal generate_tags_md helper inside register_tag.rs covering header warnings, ## Tag Categories with per-category table, optional _Last updated_ line, and graceful omission of empty auxiliary sections. The TS generator (src/generators/tags-md.ts) covers MANY more sections; for the canonical-default tags.json all aux sections are empty so the minimal port produces byte-equivalent output. When delete_tag / update_tag land (also in this batch) the helper will be promoted to a shared generators module.
  Rollback semantics: TS port has a try/catch around generateTagsMd that ROLLS BACK tags.json on markdown failure. Rust port does NOT implement rollback — markdown render is deterministic from in-memory data so failure path is fs-write failure only, which we escalate as Io error. Documented divergence.
  JSON-schema validation (Ajv against tags.schema.json) is NOT replicated in the Rust port. Upstream gates (tag-format regex + duplicate check + category existence + TagsData serde shape) collectively enforce the schema's invariants. If a future tags.schema.json clause is added that is NOT covered by upstream gates, a new explicit Rust check must be added. Documented divergence.
  Two-front-doors: dispatcher and clap CLI both call commands::register_tag::run(args_json, project_root). CLI bridge marshals positional <tag> <category> <description> args into JSON object {tag, category, description}. NO logic in bridge — JSON marshalling only.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch register-tag from the agent loop with byte-for-byte parity to the TypeScript implementation
    So that I can extend the tag vocabulary without relying on Node.js, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Auto-creates tags.json and registers a new tag in an existing category
    Given an empty project root directory with no spec/tags.json
    When I dispatch the register-tag command with tag '@api', category 'Technical Tags', and description 'API integration features'
    Then the dispatcher returns success=true
    And spec/tags.json exists after the call
    And spec/TAGS.md exists after the call
    And the Technical Tags category on disk contains a tag with name '@api' and description 'API integration features'
    And the dispatcher output contains the substring 'Successfully registered @api in Technical Tags'

  Scenario: Rejects duplicate tag across all categories
    Given spec/tags.json contains a tag '@cli' under Component Tags
    When I dispatch register-tag with tag '@cli', category 'Component Tags', and description 'CLI component'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Tag @cli is already registered in Component Tags'
    And spec/tags.json content on disk is unchanged from before the call

  Scenario: Rejects tag missing leading @ character
    Given an empty project root directory with no spec/tags.json
    When I dispatch register-tag with tag 'InvalidTag', category 'Technical Tags', and description 'Invalid format'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid tag format: "InvalidTag". Valid format is @lowercase-with-hyphens'

  Scenario: Normalises uppercase tag to lowercase and reports conversion
    Given an empty project root directory with no spec/tags.json
    When I dispatch register-tag with tag '@API-Integration', category 'Technical Tags', and description 'API features'
    Then the dispatcher returns success=true
    And the Technical Tags category on disk contains a tag with name '@api-integration'
    And the dispatcher output contains the substring 'Successfully registered @api-integration (converted from @API-Integration) in Technical Tags'

  Scenario: Rejects tag containing characters outside the allowed regex
    Given an empty project root directory with no spec/tags.json
    When I dispatch register-tag with tag '@x_underscore', category 'Technical Tags', and description 'desc'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid tag format: "@x_underscore". Valid format is @lowercase-with-hyphens'

  Scenario: Rejects unknown category with canonical Available categories list
    Given an empty project root directory with no spec/tags.json
    When I dispatch register-tag with tag '@custom', category 'NonExistent Category', and description 'desc'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid category: "NonExistent Category". Available categories: Phase Tags, Component Tags, Feature Group Tags, Technical Tags, Platform Tags, Priority Tags, Status Tags, Testing Tags, Automation Tags'

  Scenario: Matches category name case-insensitively but writes canonical on-disk name in success message
    Given an empty project root directory with no spec/tags.json
    When I dispatch register-tag with tag '@custom', category 'technical tags', and description 'desc'
    Then the dispatcher returns success=true
    And the dispatcher output contains the substring 'Successfully registered @custom in Technical Tags'
    And the Technical Tags category on disk contains a tag with name '@custom'

  Scenario: Sorts tags alphabetically within the matched category after insert
    Given spec/tags.json contains a Phase Tags category with tags '@zed', '@aaa', and '@mid' in that insertion order
    When I dispatch register-tag with tag '@bcd', category 'Phase Tags', and description 'b desc'
    Then the dispatcher returns success=true
    And the Phase Tags category on disk contains tags in the order '@aaa', '@bcd', '@mid', '@zed'

  Scenario: Preserves auxiliary top-level fields and bumps statistics.lastUpdated
    Given spec/tags.json contains populated auxiliary fields combinationExamples, usageGuidelines, and references plus an initial statistics.lastUpdated timestamp
    When I dispatch register-tag with tag '@new', category 'Technical Tags', and description 'new desc'
    Then the dispatcher returns success=true
    And spec/tags.json on disk still contains combinationExamples, usageGuidelines, and references with their original payloads
    And spec/tags.json statistics.lastUpdated on disk differs from the original initial timestamp

  Scenario: Escalates malformed tags.json as a structured parse error
    Given spec/tags.json exists but contains invalid JSON syntax
    When I dispatch register-tag against that project root
    Then the dispatcher returns success=false
    And the error message contains the substring 'Failed to parse tags.json'
    And spec/tags.json content on disk is unchanged from before the call
