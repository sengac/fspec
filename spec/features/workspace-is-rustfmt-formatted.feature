@BUG-529
@done
@BUG-475
@BUG-382
@utils
@code-quality
@rust
Feature: The Rust workspace is rustfmt-formatted and checked
  """
  cargo fmt --all --check in rust/ reported 80 unformatted files (104 by
  2026-11-03) because nothing in the quality gate ran a format check. The
  workspace is formatted with cargo fmt --all, a test-helpers test runs
  cargo fmt --all -- --check against the workspace, and the project's
  configured quality checks (fspec configure-tools) include the same check.
  Tests: rust/test-helpers/tests/bug382_workspace_is_rustfmt_formatted.rs.
  """

  Background: User Story
    As a developer working in the Rust workspace
    I want to have the whole workspace rustfmt-formatted and the format checked
    So that formatting drift cannot accumulate unnoticed

  Scenario: The workspace passes the format check
    Given the rust workspace
    When cargo fmt --all -- --check runs
    Then it reports no unformatted file
