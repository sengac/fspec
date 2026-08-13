@feature-management
@file-ops
@done
@RPC-167
Feature: Port add-architecture command to Rust
  """
  Core impl at rust/fspec-core/src/commands/add_architecture.rs reuses crate::io::gherkin::parse_feature_lenient for pre/post validation and a TS-parity line-based splice (split('\n')/Vec mutation/join('\n')), NOT AST round-trip.
  Feature path resolution reuses the basename-over-spec/features pattern (ends_with('.feature') direct, else glob_feature_files basename match) mirroring show_feature::resolve_feature_path; can share a helper with add-background.
  Two-front-doors: CLI bridge rust/fspec/src/add_architecture.rs marshals positional <feature> + <text> into JSON {feature, text} only; NO domain logic. Dispatcher and CLI call commands::add_architecture::run.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Empty or whitespace-only text MUST surface success=false with error 'Architecture text cannot be empty'
  #   2. A feature reference ending in '.feature' resolves directly under project root; a bare name resolves by basename glob over spec/features/**/*.feature; no match MUST error 'Feature file not found: <feature>'
  #   3. Feature file content MUST parse as valid Gherkin before mutation OR error 'Invalid Gherkin syntax in feature file: <msg>'; a file with no Feature line MUST error 'No Feature line found in file'
  #   4. A new doc string block ('  """' fences with each text line indented 2 spaces) is inserted immediately after the Feature line when no doc string exists
  #   5. When a Feature-line doc string already exists (paired bare """ lines) it MUST be replaced in place via splice rather than duplicated
  #   6. Mutation is line-based via split('\n')/join('\n'); the result MUST re-parse as valid Gherkin OR error 'Generated invalid Gherkin: <msg>'; success message is 'Added architecture documentation to <feature>'
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch add-architecture from the agent loop and run `fspec add-architecture` from a shell with byte-for-byte parity to the TypeScript implementation
    So that I can add or replace architecture documentation (a Feature-line doc string) without relying on Node.js, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Inserts a doc string after the Feature line when none exists
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-architecture with feature='spec/features/login.feature' and text='Uses bcrypt for password hashing'
    Then the dispatcher returns success=true
    And the dispatcher message contains 'Added architecture documentation to spec/features/login.feature'
    And the file on disk contains the line '  Uses bcrypt for password hashing'
    And the doc-string fences appear immediately after the 'Feature: Login' line

  Scenario: Inserts a multi-line doc string preserving each line
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-architecture with feature='spec/features/login.feature' and text='Uses bcrypt\nSessions in Redis'
    Then the dispatcher returns success=true
    And the file on disk contains the line '  Uses bcrypt'
    And the file on disk contains the line '  Sessions in Redis'

  Scenario: Empty text is rejected and the file is left untouched
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-architecture with feature='spec/features/login.feature' and text=''
    Then the dispatcher returns success=false
    And the error message equals 'Architecture text cannot be empty'
    And the file on disk is byte-for-byte unchanged

  Scenario: Whitespace-only text is rejected
    Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    When I dispatch add-architecture with feature='spec/features/login.feature' and text='   '
    Then the dispatcher returns success=false
    And the error message equals 'Architecture text cannot be empty'

  Scenario: Missing feature file surfaces the canonical not-found error
    Given a project root tempdir with NO spec/features/missing.feature file
    When I dispatch add-architecture with feature='spec/features/missing.feature' and text='Uses bcrypt'
    Then the dispatcher returns success=false
    And the error message equals 'Feature file not found: spec/features/missing.feature'

  Scenario: Bare feature name resolves by basename glob over spec/features
    Given a project root tempdir with spec/features/dashboard.feature containing only 'Feature: Dashboard\n  Scenario: A\n    Given x\n'
    When I dispatch add-architecture with feature='dashboard' and text='Uses a worker pool'
    Then the dispatcher returns success=true
    And the dispatcher message contains 'Added architecture documentation to dashboard'
    And the file spec/features/dashboard.feature on disk contains the line '  Uses a worker pool'

  Scenario: Replaces an existing Feature-line doc string in place
    Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  """\n  Old architecture\n  """\n  Scenario: A\n    Given x\n'
    When I dispatch add-architecture with feature='spec/features/login.feature' and text='New architecture'
    Then the dispatcher returns success=true
    And the file on disk contains the line '  New architecture'
    And the file on disk does NOT contain the line '  Old architecture'
    And the file on disk contains exactly two doc-string fence lines

  Scenario: A file with no Feature line is rejected
    Given a project root tempdir with spec/features/bad.feature containing only '# just a comment\n'
    When I dispatch add-architecture with feature='spec/features/bad.feature' and text='Uses bcrypt'
    Then the dispatcher returns success=false
    And the error message contains 'No Feature line found in file'
