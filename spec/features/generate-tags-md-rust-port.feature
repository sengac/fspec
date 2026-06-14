@wip
@RPC-236
Feature: Port generate-tags-md command to Rust

  """
  Core impl codelet/fspec-core/src/commands/generate_tags_md.rs mirrors generate_foundation_md.rs (RPC-233): exists-check spec/tags.json -> schema-validate -> parse -> render markdown -> mkdir parent -> write -> message 'Generated <out> from spec/tags.json'. Signature pub async fn run(args_json,&Path). NOTE: unlike foundation_md, tags-md.ts renders via sections.join('\n') where the final pushed section is '' so the rendered markdown ENDS WITH a single trailing newline (TS-faithful).
  New generator codelet/fspec-core/src/generators/tags_md.rs ports src/generators/tags-md.ts. New validator (analogous to foundation_schema.rs) ports tags.schema.json subset. Both require generators/mod.rs wiring (SHARED — supervisor).
  types/tags.rs::TagsData exists with categories+extra catch-all but the generator needs richer typed fields (combinationExamples, usageGuidelines, statistics, validation, references). Plan: render directly off serde_json::Value (like foundation_md renders off Value) to avoid expanding the shared TagsData struct. SHARED-FILE NOTE: types/tags.rs is owned elsewhere; rendering off Value keeps it untouched.
  CLI bridge codelet/fspec/src/generate_tags_md.rs mirrors generate_foundation_md bridge: marshals {output?} JSON; success prints '✓ <message>' exit 0; error prints 'Error: <msg>' exit 1. Help config help/configs/generate_tags_md.rs uses CommandHelpConfig (CommonError type).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When spec/tags.json is missing the command fails with error 'tags.json not found: spec/tags.json' and writes nothing
  #   2. When tags.json fails schema validation the command fails with 'tags.json has validation errors: <joined>' and writes nothing
  #   3. On success the rendered TAGS.md markdown is written verbatim (ending with a single trailing newline, TS-faithful) to spec/TAGS.md (or the --output path) and the message is 'Generated <outputRelative> from spec/tags.json'
  #   4. A relative --output path is resolved against the project root and its parent directory is created if missing
  #   5. The rendered markdown begins with the auto-generated header comment and the '# fspec Feature File Tag Registry' title, with categories rendered as tables in tags.json insertion order
  #
  # EXAMPLES:
  #   1. Given spec/tags.json exists with a valid registry, dispatching generate-tags-md writes spec/TAGS.md and returns message 'Generated spec/TAGS.md from spec/tags.json'
  #   2. Given a valid tags.json, dispatching with output='docs/TAGS.md' writes docs/TAGS.md relative to project root and returns 'Generated docs/TAGS.md from spec/tags.json'
  #   3. Given no spec/tags.json, dispatching generate-tags-md fails with 'tags.json not found: spec/tags.json' and creates no TAGS.md
  #   4. Given tags.json missing required top-level keys, dispatching fails with a 'tags.json has validation errors:' message and writes nothing
  #
  # ========================================

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the generate-tags-md command ported to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can both render spec/TAGS.md from spec/tags.json without falling back to the TS implementation

  Scenario: Generating TAGS.md from a valid tags.json writes the file and returns the success message
    Given a project root tempdir with a schema-valid spec/tags.json
    When I dispatch generate-tags-md with no output override
    Then the dispatcher returns success=true
    And the returned message is 'Generated spec/TAGS.md from spec/tags.json'
    And the bytes written to spec/TAGS.md exactly equal the rendered markdown
    And spec/TAGS.md starts with the auto-generated header comment '<!-- THIS FILE IS AUTO-GENERATED FROM spec/tags.json -->'
    And spec/TAGS.md contains the title '# fspec Feature File Tag Registry'

  Scenario: A relative output override is resolved against the project root
    Given a project root tempdir with a schema-valid spec/tags.json
    When I dispatch generate-tags-md with output='docs/TAGS.md'
    Then the dispatcher returns success=true
    And the file docs/TAGS.md is written relative to the project root
    And the returned message is 'Generated docs/TAGS.md from spec/tags.json'

  Scenario: Generation fails when spec/tags.json is missing
    Given an empty project root directory with no spec/tags.json
    When I dispatch generate-tags-md with no output override
    Then the dispatcher returns an error containing the substring 'tags.json not found: spec/tags.json'
    And the file spec/TAGS.md is not created

  Scenario: Generation fails when tags.json fails schema validation
    Given a project root tempdir with a spec/tags.json missing required top-level keys
    When I dispatch generate-tags-md with no output override
    Then the dispatcher returns an error containing the substring 'tags.json has validation errors:'
    And the file spec/TAGS.md is not created
