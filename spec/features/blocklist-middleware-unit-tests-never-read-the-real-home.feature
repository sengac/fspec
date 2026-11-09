@done
@bug-fix
@rust
@tools
@testing
@blocklist
@BUG-656
Feature: blocklist middleware unit tests read the user's real ~/.fspec/blocklist.json
  """
  The inline tests in rust/tools/src/blocklist/middleware.rs call init_blocklist, check_bash_command and check_file_path, which load the system blocklist from <HOME>/.fspec/blocklist.json (installing the default template there when it is missing) and the RLCD config from the fspec user directory. HOME and FSPEC_USER_DIR were never redirected, so the user's own rules and RLCD settings fed into the tests. Fix: the test module's isolate_home() points HOME at a per-binary temporary directory and FSPEC_USER_DIR at its .fspec directory, which holds an empty blocklist.json and an fspec-config.json that disables RLCD; every test calls it first. The guard rust/tools/tests/bug656_blocklist_middleware_tests_isolate_home.rs reads the test module's source and names every #[test] that does not call isolate_home() first. BUG-640 later moves the system blocklist path onto the shared user-directory resolver; the isolated home serves both resolutions because FSPEC_USER_DIR is <HOME>/.fspec.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every blocklist middleware unit test (rust/tools/src/blocklist/middleware.rs tests) first isolates its home: HOME is a temporary directory and FSPEC_USER_DIR is its .fspec directory, both set once per test binary
  #   2. The isolated user directory holds an empty system blocklist (blocklist.json with no rules) and an fspec-config.json that disables RLCD, so only each test's own project rules are enforced and the default template is never installed into the real home
  #   3. A guard test fails, naming the test, when any #[test] in the middleware unit tests does not isolate its home before it runs
  #
  # EXAMPLES:
  #   1. The user's ~/.fspec/blocklist.json has a rule blocking 'this_command_does_not_exist_in_any_blocklist': test_load_empty_config_when_no_files still passes, because the system blocklist it loads is the isolated empty one
  #   2. A new middleware unit test that calls check_bash_command without isolating its home makes the guard fail with its name
  #
  # ========================================
  Background: User Story
    As a developer running the codelet-tools unit tests
    I want to have the blocklist middleware unit tests read only a temporary home and user directory
    So that my own ~/.fspec/blocklist.json and fspec-config.json never change their results and are never read or written by a test run

  Scenario: The middleware unit tests load only the isolated empty system blocklist
    Given the user's real ~/.fspec/blocklist.json has a rule blocking "this_command_does_not_exist_in_any_blocklist"
    When the blocklist middleware unit tests run
    Then every test first points HOME at a temporary directory and FSPEC_USER_DIR at its ".fspec" directory
    And that directory holds a blocklist.json with no rules and an fspec-config.json that disables RLCD
    And the system blocklist the tests load is that empty one, so "this_command_does_not_exist_in_any_blocklist" is not blocked

  Scenario: A middleware unit test that does not isolate its home fails the guard
    Given a middleware unit test that calls check_bash_command without calling isolate_home first
    When the guard test reads the middleware unit tests
    Then it fails naming that test
