@feature-management
@file-ops
@done
@RPC-171
Feature: Port add-background command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/add_background.rs reuses crate::io::gherkin::parse_feature_lenient for pre/post validation and a TS-parity line-based splice (split('\n')/Vec mutation/join('\n')), NOT AST round-trip.
  Feature path resolution reuses the basename-over-spec/features pattern (ends_with('.feature') direct, else glob_feature_files basename match) mirroring show_feature::resolve_feature_path.
  Two-front-doors: CLI bridge codelet/fspec/src/add_background.rs marshals positional <feature> + <text> into JSON {feature, text} only; NO domain logic. Dispatcher and CLI call commands::add_background::run.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Empty or whitespace-only text MUST surface success=false with error 'Background text cannot be empty'
  #   2. A feature reference ending in '.feature' resolves directly under project root; a bare name resolves by basename glob over spec/features/**/*.feature; no match MUST error 'Feature file not found: <feature>'
  #   3. Feature file content MUST parse as valid Gherkin before mutation OR error 'Invalid Gherkin syntax in feature file: <msg>'; a file with no Feature line MUST error 'No Feature line found in file'
  #   4. A new Background block is inserted after any Feature-line doc string (or after the Feature line when none), titled 'Background: User Story' with each text line indented 4 spaces, surrounded by blank lines
  #   5. When a Background section already exists it MUST be replaced in place (splice over the existing range) rather than duplicated
  #   6. Mutation is line-based via split('\n')/join('\n'); the resulting content MUST re-parse as valid Gherkin OR error 'Generated invalid Gherkin: <msg>'; success message is 'Added background to <feature>'
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch add-background from the agent loop and run `fspec add-background` from a shell with byte-for-byte parity to the TypeScript implementation
    So that I can add or replace a Background (user story) section without relying on Node.js, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Adds a Background section to a feature with no existing Background
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-background with feature='spec/features/login.feature' and text='As a user\nI want to log in\nSo that I access my account'
    Then the dispatcher returns success=true
    And the dispatcher message contains 'Added background to spec/features/login.feature'
    And the file on disk contains the line '  Background: User Story'
    And the file on disk contains the line '    As a user'
    And the Background block appears after the 'Feature: Login' line and before the 'Scenario: A' line

  Scenario: Empty text is rejected and the file is left untouched
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-background with feature='spec/features/login.feature' and text=''
    Then the dispatcher returns success=false
    And the error message equals 'Background text cannot be empty'
    And the file on disk is byte-for-byte unchanged

  Scenario: Whitespace-only text is rejected
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-background with feature='spec/features/login.feature' and text='   '
    Then the dispatcher returns success=false
    And the error message equals 'Background text cannot be empty'

  Scenario: Missing feature file surfaces the canonical not-found error
    Given a project root tempdir with NO spec/features/missing.feature file
    When I dispatch add-background with feature='spec/features/missing.feature' and text='As a user'
    Then the dispatcher returns success=false
    And the error message equals 'Feature file not found: spec/features/missing.feature'

  Scenario: Bare feature name resolves by basename glob over spec/features
    Given a project root tempdir with spec/features/dashboard.feature containing only 'Feature: Dashboard\n  Scenario: A\n    Given x\n'
    When I dispatch add-background with feature='dashboard' and text='As a user\nI want a dashboard\nSo that I see an overview'
    Then the dispatcher returns success=true
    And the dispatcher message contains 'Added background to dashboard'
    And the file spec/features/dashboard.feature on disk contains the line '  Background: User Story'

  Scenario: Replaces an existing Background section in place
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n\n  Background: User Story\n    As an old user\n\n  Scenario: A\n    Given x\n'
    When I dispatch add-background with feature='spec/features/login.feature' and text='As a new user\nI want X\nSo that Y'
    Then the dispatcher returns success=true
    And the file on disk contains the line '    As a new user'
    And the file on disk does NOT contain the line '    As an old user'
    And the file on disk contains exactly one 'Background: User Story' line

  Scenario: Inserts the Background after a Feature-line doc string
    Given a project root tempdir with spec/features/api.feature containing 'Feature: API\n  """\n  Architecture notes\n  """\n\n  Scenario: A\n    Given x\n'
    When I dispatch add-background with feature='spec/features/api.feature' and text='As a developer\nI want the API\nSo that I integrate'
    Then the dispatcher returns success=true
    And the Background block appears after the closing doc-string fence and before the 'Scenario: A' line
    And the file on disk still contains the line '  Architecture notes'

  Scenario: A file with no Feature line is rejected
    Given a project root tempdir with spec/features/bad.feature containing only '# just a comment\n'
    When I dispatch add-background with feature='spec/features/bad.feature' and text='As a user'
    Then the dispatcher returns success=false
    And the error message contains 'No Feature line found in file'
