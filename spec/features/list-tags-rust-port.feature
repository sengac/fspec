@done
@RPC-251
@rust
@querying
@cli
Feature: Port list-tags command to Rust
  """
  New shared helper `io::ensure::ensure_tags_file(cwd) -> Result<TagsData, FspecCoreError>` lives alongside `ensure_prefixes_file` and uses the canonical 9-category default — Phase / Component / Feature Group / Technical / Platform / Priority / Status / Testing / Automation, in that order, each with an empty tags array. This matches `ensureTagsFile` in src/utils/ensure-files.ts:98-191 (load-or-init semantics).

  New typed `TagsData` / `TagCategory` / `Tag` shapes live in rust/fspec-core/src/types/tags.rs (new module). The `Tag` struct exposes ONLY the `tag` (a.k.a `name`) and `description` fields at the dispatcher surface — all auxiliary TS Tag-interface fields (`usage`, `scope`, `examples`, `useCases`, `whenToUse`, `criteria`, `meaning`, `testType`) are NOT projected into list-tags output. Insertion order of categories is preserved (no reordering at the dispatcher), and tags WITHIN a category are sorted alphabetically via `Ord` on the tag name (parity with the TS `a.tag.localeCompare(b.tag)` sort at src/commands/list-tags.ts:43).

  The `category` arg is an exact case-sensitive match. On miss it produces a literal `Category not found: <name>. Available categories: <comma-space-joined insertion-order names>` error string, matching src/commands/list-tags.ts:48-54.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-tags` MUST replace the NotYetPorted stub and return a real DispatchResult through the same `poll_sync_future` path used by RPC-248 (list-prefixes)
  #   2. `spec/tags.json` MUST be LOAD-OR-INIT: when missing, it is created with the canonical 9-category default and the command succeeds (parity with `ensureTagsFile`)
  #   3. Malformed `spec/tags.json` MUST escalate as `FspecCoreError::ParseJson { file: "tags.json", ... }` whose `Display` contains the substring `Failed to parse tags.json` (parity with the SyntaxError wrap-and-throw at src/utils/ensure-files.ts and the locked_file `read_or_init_json` rendering)
  #   4. Categories preserve INSERTION ORDER from the on-disk file (no alphabetisation across categories) — parity with TS `tagsData.categories.map(...)` iteration which is positional
  #   5. Tags WITHIN each category sort ALPHABETICALLY via locale-compare on the tag name (parity with src/commands/list-tags.ts:43 `.sort((a, b) => a.tag.localeCompare(b.tag))`)
  #   6. Each tag entry in the dispatcher's structured result MUST contain ONLY the `tag` and `description` fields — auxiliary Tag-interface fields are NOT projected
  #   7. The `category` argument is an EXACT case-sensitive match; passing an unknown category MUST surface an `FspecCoreError` whose message contains `Category not found: <name>. Available categories: <list>` (the available list is the comma-space-joined insertion-order names)
  #   8. The text format (default) prints `\n<name> (<N> tags)\n` headers per category; per tag either `  No tags registered` (when N=0) or `  <tag> - <description>` lines; followed by a single trailing blank line (parity with src/commands/list-tags.ts:69-83 plus the `output.log('')` at line 83)
  #   9. The JSON format (dispatcher-only) wraps the result in `{ "success": true, "categories": [...] }` with 2-space indentation
  #   10. Shared infrastructure MUST be reused: `io::ensure::ensure_tags_file` and `types::tags::TagsData` must exist as public modules so future tag commands (validate-tags, register-tag, tag-stats) inherit the same structs
  #
  # EXAMPLES:
  #   1. Dispatch list-tags against a tempdir with NO spec/tags.json → command succeeds AND spec/tags.json is created with the canonical 9-category default
  #   2. tags.json contains Phase Tags with tags ['@zed' (Z desc), '@aaa' (A desc)] in that insertion order → dispatcher output sorts them as '@aaa' then '@zed'
  #   3. tags.json contains Phase Tags THEN Automation Tags (insertion order) → dispatcher output preserves that order even though Automation < Phase alphabetically
  #   4. tags.json contains Phase Tags with a tag '@critical' carrying auxiliary fields (usage, scope, examples) → dispatcher output projects ONLY `tag` and `description` fields, NOT the auxiliary fields
  #   5. Dispatcher receives `{"category":"Phase Tags"}` → output contains Phase Tags only, not Component Tags
  #   6. Dispatcher receives `{"category":"No Such Category"}` → returns success=false with error message containing 'Category not found: No Such Category. Available categories:'
  #   7. tags.json exists but contains malformed JSON → dispatcher returns success=false with error containing 'Failed to parse tags.json'
  #   8. Dispatcher receives `{"format":"text"}` against tags.json with Phase Tags (containing '@critical' - 'Critical features') and Component Tags (empty) → output contains '\nPhase Tags (1 tags)\n', '  @critical - Critical features', '\nComponent Tags (0 tags)\n', and '  No tags registered'
  #   9. Dispatcher receives `{"format":"text"}` → output ends with a trailing blank line (matching the TS `output.log('')` after the loop)
  #   10. Dispatcher receives `{"format":"json"}` → DispatchResult.data parses as 2-space-indented JSON with shape `{"success": true, "categories": [{"name": "...", "tags": [...]}, ...]}`
  #   11. Shared modules `io::ensure::ensure_tags_file` and `types::tags::TagsData` exist and are publicly accessible from the fspec-core crate root
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-tags from the agent loop and run `fspec list-tags` from a shell with byte-for-byte parity to the TypeScript implementation
    So that I can browse registered tag categories without relying on Node.js, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Auto-creates spec/tags.json with the canonical nine-category default when missing
    Given an empty project root directory with no spec/tags.json
    When I dispatch the list-tags command against that project root with format='json'
    Then the dispatcher returns success=true
    And spec/tags.json exists after the call
    And the dispatcher result's categories array has length 9 in the order Phase Tags, Component Tags, Feature Group Tags, Technical Tags, Platform Tags, Priority Tags, Status Tags, Testing Tags, Automation Tags

  Scenario: Preserves insertion order of categories on disk (not alphabetical)
    Given spec/tags.json contains exactly two categories in the order Automation Tags then Phase Tags
    When I dispatch list-tags with format='json'
    Then the dispatcher returns success=true
    And the categories array contains exactly two entries in order Automation Tags then Phase Tags

  Scenario: Sorts tags within each category alphabetically by tag name
    Given spec/tags.json contains a Phase Tags category with tags '@zed' (description 'Z desc') and '@aaa' (description 'A desc') in that insertion order
    When I dispatch list-tags with format='json'
    Then the dispatcher returns success=true
    And the Phase Tags entry's tags array contains exactly two entries
    And the first tag entry has tag='@aaa' and description='A desc'
    And the second tag entry has tag='@zed' and description='Z desc'

  Scenario: Projects only tag and description fields, ignoring auxiliary Tag-interface fields
    Given spec/tags.json contains a Phase Tags category with a single tag whose name is '@critical', description is 'Critical features', and which also carries auxiliary fields 'usage', 'scope', and 'examples'
    When I dispatch list-tags with format='json'
    Then the dispatcher returns success=true
    And the first Phase Tags entry has tag='@critical' and description='Critical features'
    And the first Phase Tags entry does NOT contain the field 'usage'
    And the first Phase Tags entry does NOT contain the field 'scope'
    And the first Phase Tags entry does NOT contain the field 'examples'

  Scenario: Restricts output to the matching category when --category is supplied
    Given spec/tags.json contains Phase Tags (with '@critical' description 'Critical features') and Component Tags (with '@cli' description 'CLI surface')
    When I dispatch list-tags with category='Phase Tags' and format='json'
    Then the dispatcher returns success=true
    And the categories array contains exactly one entry whose name is 'Phase Tags'
    And the response data does NOT contain the substring 'Component Tags'

  Scenario: Returns structured error when --category does not match any category exactly
    Given spec/tags.json contains Phase Tags and Component Tags categories
    When I dispatch list-tags with category='No Such Category'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Category not found: No Such Category. Available categories: Phase Tags, Component Tags'

  Scenario: Escalates malformed tags.json as a structured parse error
    Given spec/tags.json exists but contains invalid JSON syntax
    When I dispatch list-tags against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse tags.json'

  Scenario: Text format renders header line and per-tag lines per category
    Given spec/tags.json contains Phase Tags (with '@critical' description 'Critical features') and Component Tags (empty)
    When I dispatch list-tags with format='text'
    Then the DispatchResult.data contains the substring 'Phase Tags (1 tags)'
    And the DispatchResult.data contains the exact line '  @critical - Critical features'
    And the DispatchResult.data contains the substring 'Component Tags (0 tags)'
    And the DispatchResult.data contains the exact line '  No tags registered'

  Scenario: Text format emits a trailing blank line after the last category
    Given spec/tags.json contains a single Phase Tags category with '@critical' (description 'Critical features')
    When I dispatch list-tags with format='text'
    Then the DispatchResult.data ends with a trailing newline character
    And the last non-empty line of the DispatchResult.data is '  @critical - Critical features'

  Scenario: JSON format emits two-space indented payload with categories array
    Given spec/tags.json contains a single Phase Tags category with '@critical' (description 'Critical features')
    When I dispatch list-tags with format='json'
    Then the DispatchResult.data parses as JSON whose root object has a 'categories' array of length 1
    And the first categories entry has name='Phase Tags' and a tags array of length 1
    And the first tag entry has tag='@critical' and description='Critical features'
    And the DispatchResult.data uses 2-space indentation

  Scenario: Shared infrastructure modules exist under rust/fspec-core for reuse by other tag commands
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/
    Then the module io::ensure::ensure_tags_file exists and is publicly accessible from the crate root
    And types::tags::TagsData exists as a public type
    And commands/list_tags.rs no longer declares the NotYetPorted stub
