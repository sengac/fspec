@done
@BUG-451
@tools
@security
@rust
Feature: RLCD config errors are reported instead of replaced by defaults
  """
  rust/tools/src/rlcd/config.rs turned an unreadable or malformed
  fspec-config.json, a non-object root and a malformed rlcd section into
  RlcdConfig::default() (logged with warn), load_rlcd_config used the
  defaults when no user directory resolved, user_dir fell back from
  FSPEC_USER_DIR to the home directory to $HOME, and pidfile_path to a
  CWD-relative .fspec. A broken config therefore silently switched the
  security stage to its defaults or another directory.
  Now user_dir, load_rlcd_config_from, load_rlcd_config and pidfile_path
  return Result. A missing file or missing rlcd section still means "not
  configured" (the documented defaults). The blocklist middleware and the
  RLCD security stage refuse the operation with the config error, the
  workflow gate rejects the command, the decision tool returns the error and
  the global supervisor reports it.
  Tests: rust/tools/tests/bug451_rlcd_config_errors_are_reported.rs.
  """

  Background: User Story
    As a user relying on the RLCD security stage
    I want to have a broken RLCD config reported as an error
    So that a typo in fspec-config.json never silently switches the security stage to its defaults or to another directory

  Scenario: Malformed RLCD config is an error
    Given fspec-config.json files with invalid JSON, a non-object root and an rlcd section with "enabled": "yes"
    When each is loaded
    Then each load fails naming the file or the rlcd section

  Scenario: A missing file or missing section is the documented default
    Given no fspec-config.json and one without an rlcd section
    When each is loaded
    Then both give the default RLCD config

  Scenario: A Bash command is refused when the RLCD config is malformed
    Given the user's fspec-config.json has a malformed rlcd section
    When a Bash command is checked by the blocklist middleware
    Then it is refused with the RLCD config error
