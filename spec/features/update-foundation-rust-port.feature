@done
@cli
@foundation-management
@rust
@RPC-312
Feature: Port update-foundation command to Rust
  """
  Core impl at rust/fspec-core/src/commands/update_foundation.rs. Validation order mirrors TS: (1) empty section, (2) projectType length 1-30, (3) problemImpact enum high|medium|low, (4) generic empty content. Reference port: add_command_to_foundation.rs (read_or_init_json + serde_json::Value mutate + write_json_atomic + generate_foundation_md::regenerate).
  Draft detection: if spec/foundation.json.draft exists, target=draft (loaded via read_or_init_json), success message 'Updated "<section>" in foundation.json.draft', NO MD regen, NO schema validation. DIVERGENCE: discover_foundation scanOnly chaining deferred (stub) — no systemReminder emitted on draft path.
  Final path: load via ensure_foundation_file (auto-creates v2.0.0 default), mutate nested path, write_json_atomic, then generate_foundation_md::regenerate(project_root). Message 'Updated "<section>" section in FOUNDATION.md'. DIVERGENCE: validateFoundationJson schema gate deferred (validate_foundation_schema is a stub). Supervisor decision noted.
  CLI bridge rust/fspec/src/update_foundation.rs: clap struct mirroring TS Commander `update-foundation <section> <content>` (both required positional). Marshals JSON {section, content} only, forwards to fspec_core. Success stdout: '✓ Updated ...' plus '  Updated: spec/foundation.json' + '  Regenerated: spec/FOUNDATION.md' (final) or '  Updated: spec/foundation.json.draft' (draft). Failure stderr 'Error: <message>' exit 1.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Empty/whitespace section name is rejected with 'Section name cannot be empty' before any file IO
  #   2. projectType has a fail-fast 1-30 character length rule that runs BEFORE the generic empty-content guard
  #   3. problemImpact has a fail-fast enum rule (high|medium|low) that runs BEFORE the generic empty-content guard
  #   4. Empty/whitespace content (for sections other than projectType/problemImpact) is rejected with 'Section content cannot be empty'
  #   5. Section names map to nested JSON paths (e.g. projectName->project.name, problemImpact->problemSpace.primaryProblem.impact, solutionOverview->solutionSpace.overview); parent objects are lazily created
  #   6. An unknown section name is rejected with 'Unknown section: "<section>". Use field names like: projectOverview, problemDefinition, etc.' and no write occurs
  #   7. When spec/foundation.json.draft exists, the draft is the write target; final foundation.json is the target otherwise (auto-created if missing)
  #   8. Final-path success writes foundation.json (2-space indent, no trailing newline) and regenerates FOUNDATION.md, returning message 'Updated "<section>" section in FOUNDATION.md'
  #   9. DIVERGENCE: the draft-path systemReminder chaining (discover_foundation scanOnly) is deferred because discover_foundation is still a Rust stub; draft success returns the message without a chained systemReminder
  #   10. DIVERGENCE: final-path schema validation (validateFoundationJson) is deferred because validate_foundation_schema is still a Rust stub; write+regenerate-MD happen without the schema gate
  #   11. Two-front-doors: CLI bridge marshals JSON {section, content} only; both dispatcher and standalone binary converge on commands::update_foundation::run
  #
  # EXAMPLES:
  #   1. Updating projectName on final foundation.json sets project.name and returns 'Updated "projectName" section in FOUNDATION.md'
  #   2. Updating problemImpact with 'urgent' fails with 'Invalid value for problemImpact: "urgent". Valid values: high, medium, low.'
  #   3. Updating projectType with a 40-char string fails with 'Invalid projectType: too long (must be 1-30 characters, got 40).'
  #   4. Updating projectVision when spec/foundation.json.draft exists writes to the draft and returns 'Updated "projectVision" in foundation.json.draft'
  #   5. Updating 'bogusSection' fails with 'Unknown section: "bogusSection"...' and foundation.json is unchanged
  #   6. Empty section name '' fails with 'Section name cannot be empty'
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the update-foundation command to Rust as a parity port
    So that the standalone Rust binary and the dispatcher can both update foundation.json section fields without falling back to TypeScript

  Scenario: Updating projectName on final foundation.json sets the nested field and regenerates FOUNDATION.md
    Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    When I dispatch update-foundation with section='projectName' and content='Acme Tool'
    Then the dispatcher returns success=true
    And the returned message is 'Updated "projectName" section in FOUNDATION.md'
    And spec/foundation.json on disk shows project.name='Acme Tool'
    And spec/FOUNDATION.md exists on disk

  Scenario: Updating problemImpact with an invalid enum value fails fast and leaves the file unchanged
    Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    When I dispatch update-foundation with section='problemImpact' and content='urgent'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid value for problemImpact: "urgent". Valid values: high, medium, low.'
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Updating projectType with a value longer than 30 characters fails fast
    Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    When I dispatch update-foundation with section='projectType' and content='this-project-type-descriptor-is-far-too-long'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid projectType: too long (must be 1-30 characters, got 44).'
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: When a draft exists the draft is the write target and no FOUNDATION.md regeneration occurs
    Given a project root tempdir with an existing spec/foundation.json.draft
    When I dispatch update-foundation with section='projectVision' and content='Ship faster'
    Then the dispatcher returns success=true
    And the returned message is 'Updated "projectVision" in foundation.json.draft'
    And spec/foundation.json.draft on disk shows project.vision='Ship faster'

  Scenario: An unknown section name is rejected and the foundation file is left unchanged
    Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    When I dispatch update-foundation with section='bogusSection' and content='whatever'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Unknown section: "bogusSection"'
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: An empty section name is rejected before any file IO
    Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    When I dispatch update-foundation with section='' and content='whatever'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Section name cannot be empty'
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: Empty content for a normal section is rejected with the generic content error
    Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    When I dispatch update-foundation with section='projectName' and content=''
    Then the dispatcher returns success=false
    And the error message contains the substring 'Section content cannot be empty'
    And spec/foundation.json on disk is byte-equal to its pre-call contents
