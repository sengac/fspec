@done
@RPC-222
Feature: Port delete-tag command to Rust

  """
  Files: codelet/fspec-core/src/commands/delete_tag.rs (replace stub); codelet/fspec-core/src/help/configs/delete_tag.rs (NEW); codelet/fspec/src/delete_tag.rs (NEW bridge); codelet/fspec-core/tests/delete_tag.rs (NEW dispatcher tests); codelet/fspec/tests/cli_delete_tag.rs (NEW CLI tests); codelet/fspec/tests/fixtures/help/delete-tag.txt (NEW captured fixture)
  Reuses shared infra: types::tags::{TagsData, TagCategory, Tag} (with #[serde(flatten)] extra preserving aux fields), io::locked_file::write_json_atomic (atomic write). NO io::ensure helper — tags.json must already exist (no auto-create, opposite of register-tag)
  Feature-file usage scan: hand-rolled recursion via std::fs::read_dir over spec/features matching *.feature; substring test contents.contains(tag) mirrors TS fileContent.includes(tag). Glob/IO failures are swallowed (best-effort) to mirror TS catch{} blocks. Kept inside the command module (no new shared dep on walkdir / glob crates)
  Inline minimal generate_tags_md helper duplicated from register_tag.rs / update_tag.rs (same shape: warning header, Tag Categories tables, optional _Last updated_ line). All three duplicates will be promoted to a shared generators module in a follow-up; intentional duplication for parallel-safe ports in Batch 7
  Divergences from TS (documented in code): (1) Ajv schema validation omitted — upstream gates enforce schema invariants; (2) outer try/catch flattened to direct FspecCoreError returns; (3) statistics.lastUpdated explicitly NOT bumped (matches TS — only register-tag bumps it); (4) fileManager.transaction() replaced with write_json_atomic
  Two-front-doors: dispatcher AND clap CLI both call commands::delete_tag::run(args_json, project_root). CLI bridge marshals positional <tag> + --force flag + --dry-run flag into JSON object {tag, force, dryRun}. NO logic in bridge — JSON marshalling only
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. spec/tags.json must already exist; delete-tag does NOT auto-create the file. Missing file → 'spec/tags.json not found' error
  #   2. Tag lookup is case-sensitive exact-match across ALL categories; first match wins
  #   3. Tag-not-found error format: 'Tag {tag} not found in registry' with no suggestion list
  #   4. Default path (no --force, no --dry-run) scans spec/features/**/*.feature for tag usage; if any matches found, blocks deletion with usage list error and 'Use --force to delete anyway' tail
  #   5. Glob/IO failures during the feature-file usage scan are SWALLOWED (best-effort scan) — both default and --force paths fall through to 'no matches' behaviour
  #   6. On successful delete, atomic-write tags.json via write_json_atomic and regenerate spec/TAGS.md from in-memory data
  #   7. statistics.lastUpdated is NOT bumped (matches TS) — auxiliary top-level fields (combinationExamples, usageGuidelines, references, statistics) round-trip untouched via #[serde(flatten)] extra
  #   8. Success message: 'Successfully deleted tag {tag} from registry' with trailing lines '  Updated: spec/tags.json' and '  Regenerated: spec/TAGS.md' (suppressed under --dry-run)
  #   9. CLI surface: positional <tag> (required), --force (flag), --dry-run (flag); NOT a category-aware command — categories are derived from where the tag currently lives
  #   10. Malformed tags.json (parse failure) escalates as structured FspecCoreError::ParseJson with reason 'Failed to parse tags.json: ...'; file on disk unchanged
  #   11. When --force is set, scan usage but emit a non-blocking warning prefix 'Warning: Tag {tag} is still used in N file(s):\n  ...' and proceed with deletion
  #   12. When --dry-run is set, skip usage scan entirely, return 'Would delete tag {tag} from category "{cat.name}"' message, perform NO disk mutation, suppress 'Updated:'/'Regenerated:' trailing lines
  #   13. When --force is combined with tag-in-use, print the warning prefix BEFORE the canonical success line in the CLI multi-line success block
  #
  # EXAMPLES:
  #   1. Happy path: spec/tags.json has tag '@deprecated' under Status Tags; no feature files reference it; dispatching delete-tag with tag '@deprecated' returns success=true, message 'Successfully deleted tag @deprecated from registry', and tags.json no longer contains the tag
  #   2. Blocked by usage: tag '@critical' is referenced by spec/features/auth.feature and spec/features/billing.feature; default dispatch returns success=false with error 'Tag @critical is used in 2 feature file(s):\n  spec/features/auth.feature\n  spec/features/billing.feature\n\nUse --force to delete anyway' and tags.json untouched
  #   3. Force override with usage: same setup but --force=true; dispatch returns success=true with warning 'Warning: Tag @critical is still used in 2 file(s):\n  spec/features/auth.feature\n  spec/features/billing.feature' AND the canonical success message; tags.json on disk no longer contains the tag
  #   4. Dry-run: tag '@critical' present in Status Tags; --dry-run=true returns success=true with message 'Would delete tag @critical from category "Status Tags"' and tags.json on disk unchanged
  #   5. Missing tags.json: empty project root → success=false with error 'spec/tags.json not found' and the file is not auto-created
  #   6. Tag not found: spec/tags.json exists but has no '@nonexistent' tag → success=false with error 'Tag @nonexistent not found in registry'
  #   7. Aux fields preserved: tags.json has combinationExamples + usageGuidelines + references + statistics.lastUpdated='1999-01-01T00:00:00.000Z' plus tag '@critical' under Phase Tags; after delete, all auxiliary fields round-trip unchanged AND statistics.lastUpdated still equals '1999-01-01T00:00:00.000Z'
  #   8. Malformed tags.json: '{ not valid json' on disk → success=false with error containing 'Failed to parse tags.json'; file content unchanged
  #   9. CLI happy path: 'fspec delete-tag @deprecated' exits 0 with stdout containing '✓ Successfully deleted tag @deprecated from registry', 'Updated: spec/tags.json', 'Regenerated: spec/TAGS.md'
  #   10. CLI dry-run: 'fspec delete-tag @critical --dry-run' exits 0 with stdout containing '✓ Would delete tag @critical from category "Status Tags"' but NO 'Updated:' / 'Regenerated:' lines
  #   11. CLI blocked by usage: 'fspec delete-tag @critical' (no --force) exits 1, stderr contains 'Error:' prefix + 'Tag @critical is used in' substring
  #   12. CLI help: 'fspec delete-tag --help' exits 0 and stdout matches the captured TypeScript fixture byte-for-byte
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch delete-tag from the agent loop with byte-for-byte parity to the TypeScript implementation
    So that I can prune obsolete tags from the registry without depending on Node.js, sharing one source of truth between the LLM dispatcher and the CLI


  Scenario: Deletes a tag and regenerates TAGS.md when no feature files reference it
    Given spec/tags.json contains a tag '@deprecated' under Status Tags
    And no feature files in the tempdir reference '@deprecated'
    When I dispatch delete-tag with tag '@deprecated' and no flags
    Then the dispatcher returns success=true
    And the dispatcher output contains the substring 'Successfully deleted tag @deprecated from registry'
    And spec/tags.json on disk no longer contains a tag named '@deprecated' in any category
    And spec/TAGS.md exists in the project root after the call


  Scenario: Blocks deletion when the tag is referenced by feature files and --force is not set
    Given spec/tags.json contains a tag '@critical' under Phase Tags
    And spec/features/auth.feature contains the substring '@critical'
    And spec/features/billing.feature contains the substring '@critical'
    When I dispatch delete-tag with tag '@critical' and no flags
    Then the dispatcher returns success=false
    And the error message contains the substring 'Tag @critical is used in 2 feature file(s):'
    And the error message contains the substring 'spec/features/auth.feature'
    And the error message contains the substring 'spec/features/billing.feature'
    And the error message contains the substring 'Use --force to delete anyway'
    And spec/tags.json content on disk is unchanged from before the call


  Scenario: Forces deletion with a warning prefix when --force is set and the tag is still in use
    Given spec/tags.json contains a tag '@critical' under Phase Tags
    And spec/features/auth.feature contains the substring '@critical'
    And spec/features/billing.feature contains the substring '@critical'
    When I dispatch delete-tag with tag '@critical' and --force
    Then the dispatcher returns success=true
    And the dispatcher output contains the substring 'Warning: Tag @critical is still used in 2 file(s):'
    And the dispatcher output contains the substring 'spec/features/auth.feature'
    And the dispatcher output contains the substring 'spec/features/billing.feature'
    And the dispatcher output contains the substring 'Successfully deleted tag @critical from registry'
    And spec/tags.json on disk no longer contains a tag named '@critical' in any category


  Scenario: Dry-run reports the intended deletion without mutating disk
    Given spec/tags.json contains a tag '@critical' under Status Tags
    When I dispatch delete-tag with tag '@critical' and --dry-run
    Then the dispatcher returns success=true
    And the dispatcher output contains the substring 'Would delete tag @critical from category "Status Tags"'
    And the dispatcher output does not contain the substring 'Updated: spec/tags.json'
    And the dispatcher output does not contain the substring 'Regenerated: spec/TAGS.md'
    And spec/tags.json content on disk is unchanged from before the call


  Scenario: Rejects request when spec/tags.json does not exist
    Given an empty project root directory with no spec/tags.json
    When I dispatch delete-tag with tag '@deprecated' and no flags
    Then the dispatcher returns success=false
    And the error message contains the substring 'spec/tags.json not found'
    And spec/tags.json was not created by the command


  Scenario: Rejects request when the tag is not found in any category
    Given spec/tags.json exists with the canonical empty category set
    When I dispatch delete-tag with tag '@nonexistent' and no flags
    Then the dispatcher returns success=false
    And the error message contains the substring 'Tag @nonexistent not found in registry'


  Scenario: Preserves auxiliary top-level fields and does NOT bump statistics.lastUpdated
    Given spec/tags.json contains a tag '@critical' under Phase Tags plus auxiliary fields combinationExamples, usageGuidelines, references, and statistics.lastUpdated set to '1999-01-01T00:00:00.000Z'
    When I dispatch delete-tag with tag '@critical' and no flags
    Then the dispatcher returns success=true
    And spec/tags.json on disk still contains combinationExamples, usageGuidelines, and references with their original payloads
    And spec/tags.json statistics.lastUpdated on disk still equals '1999-01-01T00:00:00.000Z'


  Scenario: Escalates malformed tags.json as a structured parse error
    Given spec/tags.json exists but contains invalid JSON syntax
    When I dispatch delete-tag with tag '@critical' and no flags
    Then the dispatcher returns success=false
    And the error message contains the substring 'Failed to parse tags.json'
    And spec/tags.json content on disk is unchanged from before the call


  Scenario: Suppresses 'Updated:' and 'Regenerated:' lines when dry-run succeeds
    Given spec/tags.json contains a tag '@critical' under Status Tags
    When I dispatch delete-tag with tag '@critical' and --dry-run
    Then the dispatcher output contains the substring 'Would delete tag @critical from category "Status Tags"'
    And the dispatcher output does not contain the substring 'Updated: spec/tags.json'
    And the dispatcher output does not contain the substring 'Regenerated: spec/TAGS.md'


  Scenario: Renders multi-line success block on a non-dry-run delete
    Given spec/tags.json contains a tag '@deprecated' under Status Tags
    When I dispatch delete-tag with tag '@deprecated' and no flags
    Then the dispatcher output contains the substring '✓ Successfully deleted tag @deprecated from registry'
    And the dispatcher output contains the substring 'Updated: spec/tags.json'
    And the dispatcher output contains the substring 'Regenerated: spec/TAGS.md'
